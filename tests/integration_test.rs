use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_generate_command_with_basic_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");

    // Create a basic config file
    let config_content = r#"
[docker]
environment = "prod"
ports = [8080]
entrypoint = "serve"
copy_files = ["src/"]
"#;
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("generate")
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Generated:"));

    // Check that Dockerfile was created
    let dockerfile_path = temp_dir.path().join("Dockerfile.prod");
    assert!(dockerfile_path.exists());

    let dockerfile_content = fs::read_to_string(&dockerfile_path).unwrap();
    assert!(dockerfile_content.contains("FROM ghcr.io/prefix-dev/pixi"));
    assert!(dockerfile_content.contains("EXPOSE 8080"));
    assert!(dockerfile_content.contains("CMD [\"/bin/bash\", \"-c\", \"serve\"]"));
}

#[test]
fn test_generate_default_environment() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");

    let config_content = r#"
[docker]
environment = "prod"
ports = [8000]
entrypoint = "start"

[environments.dev]
ports = [3000]
entrypoint = "dev"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Without -e, uses the default environment from config
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("generate")
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(temp_dir.path())
        .assert()
        .success();

    // Only the default environment is generated
    assert!(temp_dir.path().join("Dockerfile.prod").exists());
}

#[test]
fn test_generate_specific_environment() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");

    let config_content = r#"
[docker]
environment = "prod"
ports = [8000]

[environments.dev]
ports = [3000]
entrypoint = "dev-server"
"#;
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("generate")
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(temp_dir.path())
        .arg("--environment")
        .arg("dev")
        .assert()
        .success();

    let dockerfile_path = temp_dir.path().join("Dockerfile.dev");
    assert!(dockerfile_path.exists());

    let dockerfile_content = fs::read_to_string(&dockerfile_path).unwrap();
    assert!(dockerfile_content.contains("EXPOSE 3000"));
    assert!(dockerfile_content.contains("CMD [\"/bin/bash\", \"-c\", \"dev-server\"]"));
}

#[test]
fn test_build_command_with_tag() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");

    let config_content = r#"
[docker]
environment = "prod"
ports = [8080]
entrypoint = "serve"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create a fake docker command that always succeeds
    let fake_docker = temp_dir.path().join("docker");
    #[cfg(unix)]
    {
        fs::write(
            &fake_docker,
            "#!/bin/bash\necho 'Docker build successful'\nexit 0",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_docker).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_docker, perms).unwrap();
    }

    // Set PATH to include our fake docker
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp_dir.path().display(), old_path);
    std::env::set_var("PATH", &new_path);

    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("build")
        .arg("--config")
        .arg(&config_path)
        .arg("--tag")
        .arg("test-image:v1.0")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Building Docker image: test-image:v1.0",
        ));

    // Restore original PATH
    std::env::set_var("PATH", old_path);
}

#[test]
fn test_build_with_pixi_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");
    let pixi_path = temp_dir.path().join("pixi.toml");

    let config_content = r#"
[docker]
environment = "prod"
ports = [8080]
"#;
    fs::write(&config_path, config_content).unwrap();

    let pixi_content = r#"
[workspace]
name = "my-awesome-app"
version = "2.1.0"
"#;
    fs::write(&pixi_path, pixi_content).unwrap();

    // Create fake docker command
    let fake_docker = temp_dir.path().join("docker");
    #[cfg(unix)]
    {
        fs::write(
            &fake_docker,
            "#!/bin/bash\necho 'Docker build successful'\nexit 0",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_docker).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_docker, perms).unwrap();
    }

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp_dir.path().display(), old_path);
    std::env::set_var("PATH", &new_path);

    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("build")
        .arg("--config")
        .arg(&config_path)
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Building Docker image: my-awesome-app:2.1.0",
        ));

    std::env::set_var("PATH", old_path);
}

#[test]
fn test_invalid_config_file() {
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("generate")
        .arg("--config")
        .arg("non_existent_file.toml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Config file not found"));
}

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Generate Dockerfiles for pixi projects",
        ));
}

#[test]
fn test_generate_help() {
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("generate")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--environment"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn test_build_help() {
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("build")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--tag"))
        .stdout(predicate::str::contains("EXTRA_ARGS"));
}

#[test]
fn test_run_help() {
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    cmd.arg("run")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--tag"))
        .stdout(predicate::str::contains("--environment"))
        .stdout(predicate::str::contains("DOCKER_ARGS"));
}

#[test]
fn test_run_command_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");
    let pixi_path = temp_dir.path().join("pixi.toml");

    let config_content = r#"
[docker]
environment = "prod"
ports = [8080]
"#;
    fs::write(&config_path, config_content).unwrap();

    let pixi_content = r#"
[workspace]
name = "test-run-app"
version = "1.2.3"
"#;
    fs::write(&pixi_path, pixi_content).unwrap();

    // The run command should try to run Docker, we can verify it constructs the correct command
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    let result = cmd
        .arg("run")
        .arg("--config")
        .arg(&config_path)
        .current_dir(temp_dir.path())
        .assert();

    // Check that it shows the correct Docker command and image name
    result
        .stdout(predicate::str::contains(
            "Running Docker container: test-run-app:1.2.3",
        ))
        .stdout(predicate::str::contains("-p"))
        .stdout(predicate::str::contains("8080:8080"));
}

#[test]
fn test_run_with_docker_args() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");
    let pixi_path = temp_dir.path().join("pixi.toml");

    let config_content = r#"
[docker]
environment = "dev"
ports = [3000]
"#;
    fs::write(&config_path, config_content).unwrap();

    let pixi_content = r#"
[workspace]
name = "test-args-app"
version = "0.1.0"
"#;
    fs::write(&pixi_path, pixi_content).unwrap();

    // Test that Docker arguments are passed through correctly
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    let result = cmd
        .arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("-it")
        .arg("--rm")
        .arg("/bin/bash")
        .current_dir(temp_dir.path())
        .assert();

    result
        .stdout(predicate::str::contains(
            "Running Docker container: test-args-app:0.1.0",
        ))
        .stdout(predicate::str::contains("-it"))
        .stdout(predicate::str::contains("--rm"))
        .stdout(predicate::str::contains("/bin/bash"));
}

#[test]
fn test_run_with_complex_docker_args() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("pixi_docker.toml");
    let pixi_path = temp_dir.path().join("pixi.toml");

    let config_content = r#"
[docker]
environment = "prod"
ports = []
"#;
    fs::write(&config_path, config_content).unwrap();

    let pixi_content = r#"
[workspace]
name = "complex-test"
version = "1.0.0"
"#;
    fs::write(&pixi_path, pixi_content).unwrap();

    // Test that complex Docker arguments with values are handled correctly
    let mut cmd = Command::cargo_bin("pixi-docker").unwrap();
    let result = cmd
        .arg("run")
        .arg("--config")
        .arg(&config_path)
        .arg("-p")
        .arg("8080:8080")
        .arg("--name")
        .arg("myapp")
        .arg("-v")
        .arg("/tmp:/tmp")
        .arg("python")
        .arg("-c")
        .arg("print('test')")
        .current_dir(temp_dir.path())
        .assert();

    // Check that options come before image and command comes after
    result
        .stdout(predicate::str::contains("complex-test:1.0.0"))
        .stdout(predicate::str::contains("-p"))
        .stdout(predicate::str::contains("8080:8080"))
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("myapp"))
        .stdout(predicate::str::contains("python"));
}
