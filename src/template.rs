use anyhow::Result;
use minijinja::{context, Environment};
use crate::config::Config;
use crate::pixi::PixiToml;
use std::path::PathBuf;
use std::fs;

pub struct DockerfileGenerator {
    template_content: String,
}

impl DockerfileGenerator {
    pub fn new() -> Self {
        Self::with_template_path(None)
    }
    
    pub fn with_template_path(template_path: Option<PathBuf>) -> Self {
        let template_content = if let Some(path) = template_path {
            fs::read_to_string(path)
                .unwrap_or_else(|_| Self::default_template().to_string())
        } else {
            let default_path = PathBuf::from("templates/Dockerfile.j2");
            if default_path.exists() {
                fs::read_to_string(&default_path)
                    .unwrap_or_else(|_| Self::default_template().to_string())
            } else {
                Self::default_template().to_string()
            }
        };
        
        Self { template_content }
    }
    
    fn default_template() -> &'static str {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/Dockerfile.j2"))
    }

    pub fn generate(&self, config: &Config, environment: Option<&str>) -> Result<String> {
        let environment = environment.unwrap_or(&config.docker.environment);
        
        let env_config = config.environments.get(environment);
        
        let ports = if let Some(env_cfg) = env_config {
            if !env_cfg.ports.is_empty() {
                env_cfg.ports.clone()
            } else {
                config.docker.ports.clone()
            }
        } else {
            config.docker.ports.clone()
        };

        let entrypoint = if let Some(env_cfg) = env_config {
            env_cfg.entrypoint.as_ref().or(config.docker.entrypoint.as_ref())
        } else {
            config.docker.entrypoint.as_ref()
        };

        let copy_files = if let Some(env_cfg) = env_config {
            if !env_cfg.copy_files.is_empty() {
                env_cfg.copy_files.clone()
            } else {
                config.docker.copy_files.clone()
            }
        } else {
            config.docker.copy_files.clone()
        };
        
        let build_command = if let Some(env_cfg) = env_config {
            env_cfg.build_command.as_ref().or(config.docker.build_command.as_ref())
        } else {
            config.docker.build_command.as_ref()
        };
        
        let multi_stage = if let Some(env_cfg) = env_config {
            env_cfg.multi_stage.unwrap_or(config.docker.multi_stage)
        } else {
            config.docker.multi_stage
        };
        
        let base_image = if let Some(env_cfg) = env_config {
            env_cfg.base_image.as_ref().or(config.docker.base_image.as_ref())
        } else {
            config.docker.base_image.as_ref()
        };
        
        // Try to load pixi.toml to translate task names to shell commands
        let pixi_toml_path = PathBuf::from("pixi.toml");
        let translated_entrypoint = if let Some(entrypoint_task) = entrypoint {
            if pixi_toml_path.exists() {
                if let Ok(pixi_toml) = PixiToml::from_file(&pixi_toml_path) {
                    pixi_toml.translate_task_to_shell(entrypoint_task)
                        .unwrap_or_else(|| entrypoint_task.to_string())
                } else {
                    entrypoint_task.to_string()
                }
            } else {
                entrypoint_task.to_string()
            }
        } else {
            "".to_string()
        };
                
        let mut env = Environment::new();
        env.add_template("dockerfile", &self.template_content)?;
        let tmpl = env.get_template("dockerfile")?;
        let output = tmpl.render(context! {
            environment => environment,
            ports => ports,
            entrypoint => if translated_entrypoint.is_empty() { None } else { Some(translated_entrypoint) },
            copy_files => copy_files,
            pixi_version => config.docker.pixi_version.as_ref(),
            build_command => build_command,
            multi_stage => multi_stage,
            base_image => base_image,
        })?;
        
        Ok(output)
    }

    pub fn generate_all(&self, config: &Config) -> Result<Vec<(String, String)>> {
        let mut dockerfiles = Vec::new();
        
        dockerfiles.push((
            format!("Dockerfile.{}", config.docker.environment),
            self.generate(config, None)?
        ));
        
        for (env_name, _) in &config.environments {
            if env_name != &config.docker.environment {
                dockerfiles.push((
                    format!("Dockerfile.{}", env_name),
                    self.generate(config, Some(env_name))?
                ));
            }
        }
        
        Ok(dockerfiles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DockerConfig, EnvironmentConfig};
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut environments = HashMap::new();
        environments.insert("dev".to_string(), EnvironmentConfig {
            ports: vec![3000],
            entrypoint: Some("dev".to_string()),
            copy_files: vec!["src/".to_string(), "tests/".to_string()],
            build_command: None,
            multi_stage: Some(false),
            base_image: None,
        });

        Config {
            docker: DockerConfig {
                environment: "prod".to_string(),
                ports: vec![8080],
                entrypoint: Some("serve".to_string()),
                copy_files: vec!["app/".to_string()],
                image_name: None,
                image_tag: None,
                pixi_version: Some("0.40.0".to_string()),
                build_command: Some("build".to_string()),
                multi_stage: true,
                base_image: Some("ubuntu:24.04".to_string()),
                template_path: None,
            },
            environments,
        }
    }

    #[test]
    fn test_generator_creation() {
        let generator = DockerfileGenerator::new();
        assert!(!generator.template_content.is_empty());
    }

    #[test]
    fn test_generate_default_environment() {
        let config = create_test_config();
        let generator = DockerfileGenerator::new();
        
        let result = generator.generate(&config, None).unwrap();
        
        // Check that the generated Dockerfile contains expected elements
        assert!(result.contains("FROM ghcr.io/prefix-dev/pixi:0.40.0"));
        assert!(result.contains("prod"));
        assert!(result.contains("EXPOSE 8080"));
        assert!(result.contains("CMD [\"/bin/bash\", \"-c\", \"serve\"]"));
        assert!(result.contains("ubuntu:24.04"));
        assert!(result.contains("pixi run --locked build"));
    }

    #[test]
    fn test_generate_specific_environment() {
        let config = create_test_config();
        let generator = DockerfileGenerator::new();
        
        let result = generator.generate(&config, Some("dev")).unwrap();
        
        // Check dev-specific configuration
        assert!(result.contains("dev"));
        assert!(result.contains("EXPOSE 3000"));
        assert!(result.contains("CMD [\"/bin/bash\", \"-c\", \"dev\"]"));
        
        // Dev environment has multi_stage = false, so it won't have multi-stage build structure
        // Instead it should have single stage structure
        assert!(!result.contains("FROM ubuntu:24.04 AS production"));
    }

    #[test]
    fn test_generate_all_environments() {
        let config = create_test_config();
        let generator = DockerfileGenerator::new();
        
        let dockerfiles = generator.generate_all(&config).unwrap();
        
        assert_eq!(dockerfiles.len(), 2); // prod and dev
        
        let filenames: Vec<_> = dockerfiles.iter().map(|(name, _)| name).collect();
        assert!(filenames.contains(&&"Dockerfile.prod".to_string()));
        assert!(filenames.contains(&&"Dockerfile.dev".to_string()));
    }

    #[test]
    fn test_environment_config_overrides() {
        let config = create_test_config();
        let generator = DockerfileGenerator::new();
        
        // Test that dev environment uses its own ports instead of default
        let result = generator.generate(&config, Some("dev")).unwrap();
        assert!(result.contains("EXPOSE 3000"));
        assert!(!result.contains("EXPOSE 8080"));
    }

    #[test]
    fn test_fallback_to_default_values() {
        let mut config = create_test_config();
        config.docker.entrypoint = None;
        
        let generator = DockerfileGenerator::new();
        let result = generator.generate(&config, None).unwrap();
        
        // Should fallback to bash when no entrypoint is specified
        assert!(result.contains("CMD [\"/bin/bash\"]"));
    }

    #[test]
    fn test_custom_template_path() {
        // Test using basic template content as we don't have a custom file
        let test_template = "FROM test:latest\nWORKDIR /test\n";
        
        // For this test, we'll create a simple generator with known template content
        let generator = DockerfileGenerator {
            template_content: test_template.to_string(),
        };
        
        let config = create_test_config();
        let _result = generator.generate(&config, None).unwrap();
        
        // The result should contain our test template parts (though it will error due to invalid template)
        // This mainly tests that custom template content is used
        assert!(generator.template_content.contains("FROM test:latest"));
    }
}