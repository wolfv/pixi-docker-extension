mod config;
mod pixi;
mod template;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use config::Config;
use pixi::PixiToml;
use template::DockerfileGenerator;

#[derive(Parser)]
#[command(name = "pixi-docker")]
#[command(about = "Generate Dockerfiles for pixi projects", long_about = None)]
struct Cli {
    /// Configuration file
    #[arg(short, long, default_value = "pixi_docker.toml", global = true)]
    config: PathBuf,

    /// Target environment
    #[arg(short, long, global = true)]
    environment: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate Dockerfiles without building
    Generate {
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Generate and build a Docker image
    Build {
        /// Custom image tag (default: from pixi.toml)
        #[arg(short = 't', long)]
        tag: Option<String>,

        /// Additional arguments passed to 'docker build'
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Vec<String>,
    },
    /// Run a Docker container
    Run {
        /// Custom image tag (default: from pixi.toml)
        #[arg(short = 't', long)]
        tag: Option<String>,

        /// Additional arguments passed to 'docker run'
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        docker_args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.config.exists() {
        anyhow::bail!("Config file not found: {:?}", cli.config);
    }

    let config = Config::from_file(&cli.config)?;
    let environment = cli
        .environment
        .as_deref()
        .unwrap_or(&config.docker.environment);

    match cli.command {
        Some(Commands::Generate { output }) => {
            generate_dockerfiles(&config, environment, output)?;
        }
        Some(Commands::Build { tag, extra_args }) => {
            build_docker_image(&config, environment, tag, extra_args)?;
        }
        Some(Commands::Run { tag, docker_args }) => {
            run_docker_container(&config, environment, tag, docker_args)?;
        }
        None => {
            generate_dockerfiles(&config, environment, PathBuf::from("."))?;
        }
    }

    Ok(())
}

/// Resolve the image tag from CLI, config, or pixi.toml
fn resolve_image_tag(config: &Config, environment: &str, cli_tag: Option<String>) -> String {
    if let Some(tag) = cli_tag {
        return tag;
    }

    let pixi_toml_path = PathBuf::from("pixi.toml");
    let pixi_toml = pixi_toml_path
        .exists()
        .then(|| PixiToml::from_file(&pixi_toml_path).ok())
        .flatten();

    let name = config
        .docker
        .image_name
        .as_ref()
        .or_else(|| pixi_toml.as_ref().and_then(|p| p.get_name()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "pixi-app".to_string());

    let version = config
        .docker
        .image_tag
        .as_ref()
        .or_else(|| pixi_toml.as_ref().and_then(|p| p.get_version()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| environment.to_string());

    format!("{}:{}", name, version)
}

fn generate_dockerfiles(config: &Config, environment: &str, output_dir: PathBuf) -> Result<()> {
    let generator = if let Some(template_path) = &config.docker.template_path {
        DockerfileGenerator::with_template_path(Some(PathBuf::from(template_path)))
    } else {
        DockerfileGenerator::new()
    };

    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }

    let dockerfile_content = generator.generate(config, Some(environment))?;
    let filename = format!("Dockerfile.{}", environment);
    let output_path = output_dir.join(&filename);
    fs::write(&output_path, dockerfile_content)?;
    println!("Generated: {}", output_path.display());

    Ok(())
}

fn build_docker_image(
    config: &Config,
    environment: &str,
    tag: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    // First generate the Dockerfile
    let generator = if let Some(template_path) = &config.docker.template_path {
        DockerfileGenerator::with_template_path(Some(PathBuf::from(template_path)))
    } else {
        DockerfileGenerator::new()
    };
    let dockerfile_content = generator.generate(config, Some(environment))?;
    let dockerfile_name = format!("Dockerfile.{}", environment);
    fs::write(&dockerfile_name, &dockerfile_content)?;
    println!("Generated: {}", dockerfile_name);

    let image_tag = resolve_image_tag(config, environment, tag);

    // Build the Docker command
    let mut docker_cmd = Command::new("docker");
    docker_cmd
        .arg("build")
        .arg("-t")
        .arg(&image_tag)
        .arg("-f")
        .arg(&dockerfile_name);

    for arg in extra_args {
        docker_cmd.arg(arg);
    }

    docker_cmd.arg(".");

    println!("Building Docker image: {}", image_tag);
    println!("Running: {:?}", docker_cmd);

    let status = docker_cmd.status()?;
    if !status.success() {
        anyhow::bail!("Docker build failed with exit code: {:?}", status.code());
    }

    println!("Successfully built Docker image: {}", image_tag);
    Ok(())
}

fn run_docker_container(
    config: &Config,
    environment: &str,
    tag: Option<String>,
    docker_args: Vec<String>,
) -> Result<()> {
    let image_tag = resolve_image_tag(config, environment, tag);

    let mut docker_cmd = Command::new("docker");
    docker_cmd.arg("run");

    // If no args provided, add sensible defaults (port mapping + interactive)
    if docker_args.is_empty() {
        let env_config = config.environments.get(environment);
        let ports = env_config
            .filter(|e| !e.ports.is_empty())
            .map(|e| &e.ports)
            .unwrap_or(&config.docker.ports);

        for port in ports {
            docker_cmd.arg("-p").arg(format!("{}:{}", port, port));
        }
        docker_cmd.arg("-it");
    } else {
        // Pass all args through - user is responsible for correct ordering
        for arg in &docker_args {
            docker_cmd.arg(arg);
        }
    }

    docker_cmd.arg(&image_tag);

    println!("Running Docker container: {}", image_tag);
    println!("Command: {:?}", docker_cmd);

    let status = docker_cmd.status()?;
    if !status.success() {
        anyhow::bail!("Docker run failed with exit code: {:?}", status.code());
    }

    Ok(())
}
