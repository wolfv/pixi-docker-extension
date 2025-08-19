use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub docker: DockerConfig,
    #[serde(default)]
    pub environments: HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DockerConfig {
    pub environment: String,
    #[serde(default)]
    pub ports: Vec<u16>,
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub copy_files: Vec<String>,
    pub image_name: Option<String>,
    pub image_tag: Option<String>,
    pub pixi_version: Option<String>,
    pub build_command: Option<String>,
    #[serde(default = "default_multi_stage")]
    pub multi_stage: bool,
    pub base_image: Option<String>,
    pub template_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct EnvironmentConfig {
    #[serde(default)]
    pub ports: Vec<u16>,
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub copy_files: Vec<String>,
    pub build_command: Option<String>,
    pub multi_stage: Option<bool>,
    pub base_image: Option<String>,
}

fn default_multi_stage() -> bool {
    true
}

impl Config {
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_basic_config() {
        let path = PathBuf::from("tests/fixtures/basic_config.toml");
        let config = Config::from_file(&path).unwrap();

        assert_eq!(config.docker.environment, "prod");
        assert_eq!(config.docker.ports, vec![8080]);
        assert_eq!(config.docker.entrypoint, Some("serve".to_string()));
        assert_eq!(config.docker.copy_files, vec!["src/"]);
        assert_eq!(config.docker.pixi_version, Some("0.40.0".to_string()));
        assert_eq!(config.docker.build_command, Some("build".to_string()));
        assert_eq!(config.docker.multi_stage, true);
        assert_eq!(config.docker.base_image, Some("ubuntu:24.04".to_string()));
    }

    #[test]
    fn test_load_multi_env_config() {
        let path = PathBuf::from("tests/fixtures/multi_env_config.toml");
        let config = Config::from_file(&path).unwrap();

        assert_eq!(config.docker.environment, "prod");
        assert_eq!(config.docker.ports, vec![8000]);

        // Check dev environment
        let dev_env = config.environments.get("dev").unwrap();
        assert_eq!(dev_env.ports, vec![3000, 3001]);
        assert_eq!(dev_env.entrypoint, Some("dev".to_string()));
        assert_eq!(dev_env.copy_files, vec!["app/", "tests/"]);
        assert_eq!(dev_env.multi_stage, Some(false));

        // Check test environment
        let test_env = config.environments.get("test").unwrap();
        assert_eq!(test_env.ports, vec![]);
        assert_eq!(test_env.entrypoint, Some("test".to_string()));
        assert_eq!(test_env.build_command, Some("test-build".to_string()));
    }

    #[test]
    fn test_default_multi_stage() {
        assert_eq!(default_multi_stage(), true);
    }

    #[test]
    fn test_config_from_string() {
        let toml_str = r#"
            [docker]
            environment = "production"
            ports = [80, 443]
            entrypoint = "app"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.docker.environment, "production");
        assert_eq!(config.docker.ports, vec![80, 443]);
        assert_eq!(config.docker.entrypoint, Some("app".to_string()));
        assert_eq!(config.docker.multi_stage, true); // default value
    }

    #[test]
    fn test_invalid_config() {
        let path = PathBuf::from("non_existent_file.toml");
        let result = Config::from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_with_template_path() {
        let toml_str = r#"
            [docker]
            environment = "prod"
            ports = [8080]
            template_path = "custom/template.j2"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.docker.template_path,
            Some("custom/template.j2".to_string())
        );
    }
}
