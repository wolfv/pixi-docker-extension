use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize)]
pub struct PixiToml {
    #[serde(rename = "workspace")]
    pub workspace: Option<WorkspaceConfig>,
    #[serde(rename = "project")]
    pub project: Option<ProjectConfig>,
    #[serde(default)]
    pub tasks: HashMap<String, TaskValue>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskValue {
    Simple(String),
    Complex(TaskConfig),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskConfig {
    pub cmd: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub version: Option<String>,
}

impl PixiToml {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let pixi_toml: PixiToml = toml::from_str(&content)?;
        Ok(pixi_toml)
    }
    
    pub fn get_name(&self) -> Option<&String> {
        self.workspace.as_ref()
            .and_then(|w| w.name.as_ref())
            .or_else(|| self.project.as_ref().and_then(|p| p.name.as_ref()))
    }
    
    pub fn get_version(&self) -> Option<&String> {
        self.workspace.as_ref()
            .and_then(|w| w.version.as_ref())
            .or_else(|| self.project.as_ref().and_then(|p| p.version.as_ref()))
    }
    
    pub fn get_task_command(&self, task_name: &str) -> Option<String> {
        self.tasks.get(task_name).map(|task| match task {
            TaskValue::Simple(cmd) => cmd.clone(),
            TaskValue::Complex(config) => config.cmd.clone(),
        })
    }
    
    pub fn translate_task_to_shell(&self, task_name: &str) -> Option<String> {
        if let Some(command) = self.get_task_command(task_name) {
            Some(command)
        } else {
            // If task not found, return the task name as-is
            // (could be a shell command already)
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_workspace_config() {
        let path = PathBuf::from("tests/fixtures/test_pixi.toml");
        let pixi = PixiToml::from_file(&path).unwrap();
        
        assert_eq!(pixi.get_name(), Some(&"test-app".to_string()));
        assert_eq!(pixi.get_version(), Some(&"2.3.4".to_string()));
    }

    #[test]
    fn test_project_config() {
        let toml_str = r#"
            [project]
            name = "my-project"
            version = "1.0.0"
        "#;
        
        let pixi: PixiToml = toml::from_str(toml_str).unwrap();
        assert_eq!(pixi.get_name(), Some(&"my-project".to_string()));
        assert_eq!(pixi.get_version(), Some(&"1.0.0".to_string()));
        // assert_eq!(pixi.get_image_tag(), "my-project:1.0.0");
    }

    #[test]
    fn test_workspace_takes_precedence() {
        let toml_str = r#"
            [workspace]
            name = "workspace-name"
            version = "2.0.0"
            
            [project]
            name = "project-name"
            version = "1.0.0"
        "#;
        
        let pixi: PixiToml = toml::from_str(toml_str).unwrap();
        assert_eq!(pixi.get_name(), Some(&"workspace-name".to_string()));
        assert_eq!(pixi.get_version(), Some(&"2.0.0".to_string()));
    }

    #[test]
    fn test_default_values() {
        let toml_str = r#"
            [dependencies]
            python = "3.11"
        "#;
        
        let pixi: PixiToml = toml::from_str(toml_str).unwrap();
        assert_eq!(pixi.get_name(), None);
        assert_eq!(pixi.get_version(), None);
        // assert_eq!(pixi.get_image_tag(), "pixi-app:latest");
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
            [workspace]
            name = "my-app"
        "#;
        
        let pixi: PixiToml = toml::from_str(toml_str).unwrap();
        assert_eq!(pixi.get_name(), Some(&"my-app".to_string()));
        assert_eq!(pixi.get_version(), None);
        // assert_eq!(pixi.get_image_tag(), "my-app:latest");
    }

    #[test]
    fn test_invalid_file() {
        let path = PathBuf::from("non_existent.toml");
        let result = PixiToml::from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_parsing() {
        let toml_str = r#"
            [workspace]
            name = "test-tasks"
            
            [tasks]
            simple-task = "echo hello"
            server = "python src/main.py"
            build = "cargo build"
        "#;
        
        let pixi: PixiToml = toml::from_str(toml_str).unwrap();
        
        // Test task command extraction
        assert_eq!(pixi.get_task_command("simple-task"), Some("echo hello".to_string()));
        assert_eq!(pixi.get_task_command("server"), Some("python src/main.py".to_string()));
        assert_eq!(pixi.get_task_command("build"), Some("cargo build".to_string()));
        assert_eq!(pixi.get_task_command("nonexistent"), None);
        
        // Test task translation
        assert_eq!(pixi.translate_task_to_shell("server"), Some("python src/main.py".to_string()));
        assert_eq!(pixi.translate_task_to_shell("nonexistent"), None);
    }

    #[test]
    fn test_task_translation_fallback() {
        let toml_str = r#"
            [workspace]
            name = "minimal"
        "#;
        
        let pixi: PixiToml = toml::from_str(toml_str).unwrap();
        
        // Should return None for non-existent tasks
        assert_eq!(pixi.translate_task_to_shell("some-command"), None);
    }
}