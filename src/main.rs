mod config;
mod template;
mod pixi;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::process::Command;

use config::Config;
use template::DockerfileGenerator;
use pixi::PixiToml;

#[derive(Parser)]
#[command(name = "pixi-docker")]
#[command(about = "Generate Dockerfiles for pixi projects", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Generate {
        #[arg(short, long, default_value = "pixi_docker.toml")]
        config: PathBuf,
        
        #[arg(short, long)]
        environment: Option<String>,
        
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        
        #[arg(short, long)]
        all: bool,
    },
    Build {
        #[arg(short, long, default_value = "pixi_docker.toml")]
        config: PathBuf,
        
        #[arg(short, long)]
        environment: Option<String>,
        
        #[arg(short = 't', long)]
        tag: Option<String>,
        
        #[arg(long)]
        no_cache: bool,
        
        #[arg(long)]
        platform: Option<String>,
        
        #[arg(trailing_var_arg = true)]
        extra_args: Vec<String>,
    },
    Run {
        #[arg(short, long, default_value = "pixi_docker.toml")]
        config: PathBuf,
        
        #[arg(short, long)]
        environment: Option<String>,
        
        #[arg(short = 't', long)]
        tag: Option<String>,
        
        #[arg(
            trailing_var_arg = true, 
            allow_hyphen_values = true,
            help = "Additional arguments to pass to 'docker run'. Use -- to separate if needed."
        )]
        docker_args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Generate { config, environment, output, all }) => {
            generate_dockerfiles(config, environment, output, all)?;
        }
        Some(Commands::Build { config, environment, tag, no_cache, platform, extra_args }) => {
            build_docker_image(config, environment, tag, no_cache, platform, extra_args)?;
        }
        Some(Commands::Run { config, environment, tag, docker_args }) => {
            run_docker_container(config, environment, tag, docker_args)?;
        }
        None => {
            let config_path = PathBuf::from("pixi_docker.toml");
            if config_path.exists() {
                generate_dockerfiles(config_path, None, PathBuf::from("."), true)?;
            } else {
                eprintln!("No pixi_docker.toml found. Use 'pixi-docker generate', 'pixi-docker build', or 'pixi-docker run' with options.");
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}

fn generate_dockerfiles(
    config_path: PathBuf, 
    environment: Option<String>,
    output_dir: PathBuf,
    all: bool
) -> Result<()> {
    if !config_path.exists() {
        anyhow::bail!("Config file not found: {:?}", config_path);
    }
    
    let config = Config::from_file(&config_path)?;
    let generator = if let Some(template_path) = &config.docker.template_path {
        DockerfileGenerator::with_template_path(Some(PathBuf::from(template_path)))
    } else {
        DockerfileGenerator::new()
    };
    
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }
    
    if all || environment.is_none() {
        let dockerfiles = generator.generate_all(&config)?;
        for (filename, content) in dockerfiles {
            let output_path = output_dir.join(&filename);
            fs::write(&output_path, content)?;
            println!("Generated: {}", output_path.display());
        }
    } else if let Some(env) = environment {
        let dockerfile_content = generator.generate(&config, Some(&env))?;
        let filename = format!("Dockerfile.{}", env);
        let output_path = output_dir.join(&filename);
        fs::write(&output_path, dockerfile_content)?;
        println!("Generated: {}", output_path.display());
    }
    
    Ok(())
}

fn build_docker_image(
    config_path: PathBuf,
    environment: Option<String>,
    tag: Option<String>,
    no_cache: bool,
    platform: Option<String>,
    extra_args: Vec<String>,
) -> Result<()> {
    if !config_path.exists() {
        anyhow::bail!("Config file not found: {:?}", config_path);
    }
    
    let config = Config::from_file(&config_path)?;
    let environment = environment.as_deref().unwrap_or(&config.docker.environment);
    
    // First generate the Dockerfile
    let generator = if let Some(template_path) = &config.docker.template_path {
        DockerfileGenerator::with_template_path(Some(PathBuf::from(template_path)))
    } else {
        DockerfileGenerator::new()
    };
    let dockerfile_content = generator.generate(&config, Some(environment))?;
    let dockerfile_name = format!("Dockerfile.{}", environment);
    fs::write(&dockerfile_name, dockerfile_content)?;
    println!("Generated: {}", dockerfile_name);
    
    // Determine the image tag
    let image_tag = if let Some(tag) = tag {
        tag
    } else {
        // Try to read from pixi.toml
        let pixi_toml_path = PathBuf::from("pixi.toml");
        if pixi_toml_path.exists() {
            let pixi_toml = PixiToml::from_file(&pixi_toml_path)?;
            let name = config.docker.image_name.as_ref()
                .or_else(|| pixi_toml.get_name())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "pixi-app".to_string());
            
            let version = config.docker.image_tag.as_ref()
                .or_else(|| pixi_toml.get_version())
                .map(|s| s.to_string())
                .unwrap_or_else(|| environment.to_string());
            
            format!("{}:{}", name, version)
        } else {
            let name = config.docker.image_name.as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "pixi-app".to_string());
            
            let version = config.docker.image_tag.as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| environment.to_string());
            
            format!("{}:{}", name, version)
        }
    };
    
    // Build the Docker command
    let mut docker_cmd = Command::new("docker");
    docker_cmd.arg("build");
    docker_cmd.arg("-t").arg(&image_tag);
    docker_cmd.arg("-f").arg(&dockerfile_name);
    
    if no_cache {
        docker_cmd.arg("--no-cache");
    }
    
    if let Some(platform) = platform {
        docker_cmd.arg("--platform").arg(platform);
    }
    
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
    config_path: PathBuf,
    environment: Option<String>,
    tag: Option<String>,
    docker_args: Vec<String>,
) -> Result<()> {
    if !config_path.exists() {
        anyhow::bail!("Config file not found: {:?}", config_path);
    }
    
    let config = Config::from_file(&config_path)?;
    let environment = environment.as_deref().unwrap_or(&config.docker.environment);
    
    // Determine the image tag to run
    let image_tag = if let Some(tag) = tag {
        tag
    } else {
        // Try to read from pixi.toml
        let pixi_toml_path = PathBuf::from("pixi.toml");
        if pixi_toml_path.exists() {
            let pixi_toml = PixiToml::from_file(&pixi_toml_path)?;
            let name = config.docker.image_name.as_ref()
                .or_else(|| pixi_toml.get_name())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "pixi-app".to_string());
            
            let version = config.docker.image_tag.as_ref()
                .or_else(|| pixi_toml.get_version())
                .map(|s| s.to_string())
                .unwrap_or_else(|| environment.to_string());
            
            format!("{}:{}", name, version)
        } else {
            let name = config.docker.image_name.as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "pixi-app".to_string());
            
            let version = config.docker.image_tag.as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| environment.to_string());
            
            format!("{}:{}", name, version)
        }
    };
    
    // Build the Docker run command
    let mut docker_cmd = Command::new("docker");
    docker_cmd.arg("run");
    
    // Parse docker_args to separate Docker options from the container command
    // We'll use a simple heuristic: once we find an argument that doesn't start with -
    // and isn't a value for a previous option, we treat it and everything after as the command
    let mut docker_options = Vec::new();
    let mut container_command = Vec::new();
    let mut i = 0;
    let mut expecting_value = false;
    
    while i < docker_args.len() {
        let arg = &docker_args[i];
        
        if container_command.is_empty() {
            if arg.starts_with('-') {
                // This is a Docker option
                docker_options.push(arg.clone());
                
                // Check if this option expects a value
                // Common Docker run options that take values
                if arg == "-p" || arg == "--publish" || 
                   arg == "-v" || arg == "--volume" ||
                   arg == "-e" || arg == "--env" ||
                   arg == "--name" || arg == "--network" ||
                   arg == "--user" || arg == "-u" ||
                   arg == "--workdir" || arg == "-w" ||
                   arg == "--entrypoint" || arg == "--hostname" ||
                   arg == "--memory" || arg == "-m" ||
                   arg == "--cpus" || arg == "--label" || arg == "-l" {
                    expecting_value = true;
                } else if arg.contains('=') {
                    // Option with = sign like --name=foo
                    expecting_value = false;
                } else {
                    expecting_value = false;
                }
            } else if expecting_value {
                // This is a value for the previous option
                docker_options.push(arg.clone());
                expecting_value = false;
            } else {
                // This is the start of the container command
                container_command.push(arg.clone());
            }
        } else {
            // Everything after the first command argument is part of the command
            container_command.push(arg.clone());
        }
        
        i += 1;
    }
    
    // Add automatic port mapping if no custom args are provided
    if docker_args.is_empty() {
        // Get ports for this environment
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
        
        // Add port mappings
        for port in ports {
            docker_cmd.arg("-p").arg(format!("{}:{}", port, port));
        }
        
        // Add interactive mode for better UX
        docker_cmd.arg("-it");
    } else {
        // Add Docker options before the image name
        for option in docker_options {
            docker_cmd.arg(option);
        }
    }
    
    // Add the image name
    docker_cmd.arg(&image_tag);
    
    // Add container command after the image name
    for cmd in container_command {
        docker_cmd.arg(cmd);
    }
    
    println!("Running Docker container: {}", image_tag);
    println!("Command: {:?}", docker_cmd);
    
    let status = docker_cmd.status()?;
    
    if !status.success() {
        anyhow::bail!("Docker run failed with exit code: {:?}", status.code());
    }
    
    Ok(())
}
