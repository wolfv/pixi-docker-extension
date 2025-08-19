# pixi-docker

A pixi plugin for generating and building Docker containers for pixi projects.

## Features

- Generate Dockerfiles from configuration
- Multi-stage builds for optimized production images
- Support for multiple environments (dev, prod, test, etc.)
- Integration with pixi.toml for automatic image naming
- Build Docker images directly with configurable options
- Customizable Jinja2 templates

## Installation

```bash
cargo build --release
# Copy target/release/pixi-docker to your PATH
```

## Quick Start

1. Create a `pixi_docker.toml` configuration file:

```toml
[docker]
environment = "prod"
ports = [8000]
entrypoint = "start-server"
copy_files = ["src/", "static/"]
pixi_version = "0.40.0"
build_command = "build"
multi_stage = true
base_image = "ubuntu:24.04"

[environments.dev]
ports = [8000, 3000]
entrypoint = "dev-server"
copy_files = ["src/", "static/", "tests/"]
multi_stage = false

[environments.test]
entrypoint = "test"
build_command = "test-build"
```

2. Generate Dockerfiles:

```bash
# Generate all environments
pixi-docker generate

# Generate specific environment
pixi-docker generate -e dev

# Generate to custom output directory
pixi-docker generate -o docker/
```

3. Build Docker images:

```bash
# Build with automatic naming from pixi.toml
pixi-docker build

# Build with custom tag
pixi-docker build -t my-app:v1.0

# Build specific environment
pixi-docker build -e prod

# Build with extra options
pixi-docker build --no-cache --platform linux/amd64
```

## Commands

### generate

Generate Dockerfiles from configuration.

```bash
pixi-docker generate [OPTIONS]

Options:
  -c, --config <CONFIG>            Configuration file [default: pixi_docker.toml]
  -e, --environment <ENVIRONMENT> Generate for specific environment
  -o, --output <OUTPUT>            Output directory [default: .]
  -a, --all                        Generate all environments
```

### build

Generate Dockerfile and build Docker image.

```bash
pixi-docker build [OPTIONS] [EXTRA_ARGS]...

Options:
  -c, --config <CONFIG>            Configuration file [default: pixi_docker.toml]
  -e, --environment <ENVIRONMENT> Build specific environment
  -t, --tag <TAG>                  Custom image tag
      --no-cache                   Build without cache
      --platform <PLATFORM>        Target platform
```

### run

Run Docker container with automatic configuration.

```bash
pixi-docker run [OPTIONS] [DOCKER_ARGS]...

Options:
  -c, --config <CONFIG>            Configuration file [default: pixi_docker.toml]
  -e, --environment <ENVIRONMENT> Run specific environment
  -t, --tag <TAG>                  Custom image tag
```

The run command automatically:
- Determines the correct image tag from pixi.toml
- Maps ports based on environment configuration
- Adds `-it` flags for interactive mode (if no custom args provided)
- Forwards any additional arguments to `docker run`

Examples:
```bash
# Basic run with automatic configuration
pixi-docker run

# Interactive shell access
pixi-docker run -it /bin/bash

# Run with additional Docker flags
pixi-docker run --rm --name myapp -it /bin/bash

# Alternative syntax with -- separator (optional)
pixi-docker run -- --rm --name myapp -it /bin/bash
```

## Configuration

### Docker Section

The `[docker]` section defines default settings:

- `environment`: Default environment to use
- `ports`: List of ports to expose
- `entrypoint`: Command to run in container
- `copy_files`: Files/directories to copy into image
- `pixi_version`: Pixi version to use (default: "latest")
- `build_command`: Command to run during build phase
- `multi_stage`: Enable multi-stage builds (default: true)
- `base_image`: Base image for production stage
- `image_name`: Override default image name
- `image_tag`: Override default image tag

### Environment Sections

Environment-specific overrides in `[environments.<name>]`:

```toml
[environments.dev]
ports = [3000, 8000]
entrypoint = "dev-server"
multi_stage = false
build_command = "dev-build"
```

## Templates

The plugin uses Jinja2 templates located in `templates/Dockerfile.j2`. You can customize the template by editing this file or providing a custom template path.

### Available Template Variables

- `environment`: Current environment name
- `ports`: List of ports to expose
- `entrypoint`: Entrypoint command
- `copy_files`: Files to copy
- `pixi_version`: Pixi version
- `build_command`: Build command
- `multi_stage`: Whether to use multi-stage build
- `base_image`: Base image for production stage

## Examples

See the `examples/` directory for sample configurations:

- `examples/pixi_docker.toml`: Full configuration example
- `examples/pixi.toml`: Sample pixi project configuration

## Project Structure

```
pixi-docker-plugin/
├── src/
│   ├── main.rs          # CLI interface
│   ├── config.rs        # Configuration parsing
│   ├── pixi.rs          # Pixi.toml parsing
│   └── template.rs      # Dockerfile generation
├── templates/
│   └── Dockerfile.j2    # Default Dockerfile template
├── examples/            # Example configurations
├── tests/
│   ├── fixtures/        # Test data
│   └── integration_test.rs
└── Cargo.toml

```

## Testing

Run the test suite:

```bash
cargo test
```

This runs:
- Unit tests for all modules
- Integration tests for CLI commands
- Template generation tests
- Configuration parsing tests

## License

This project is licensed under the MIT License.