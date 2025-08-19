# Pixi Docker Plugin Example

This directory contains a complete example showing how to use the pixi-docker plugin with a real project.

## Project Structure

```
examples/
├── src/
│   └── main.py              # Simple Python web server
├── static/
│   └── index.html           # Static HTML page
├── tests/
│   └── test_main.py         # Example tests
├── pixi.toml               # Pixi project configuration
├── pixi_docker.toml        # Docker plugin configuration
└── README.md               # This file
```

## Quick Demo

Run the interactive demo:

```bash
./demo.sh
```

This will:
1. Show the current configuration
2. Generate Dockerfiles for all environments
3. Display the generated files
4. Show example build commands

## Manual Usage

### Generate Dockerfiles

```bash
# Generate all environments
pixi-docker generate --all

# Generate specific environment
pixi-docker generate -e prod

# Generate to custom directory
pixi-docker generate -o docker/
```

### Build Docker Images

```bash
# Build with automatic naming (my-pixi-app:1.0.0)
pixi-docker build

# Build with custom tag
pixi-docker build -t my-custom-name:v1.0

# Build specific environment
pixi-docker build -e dev

# Build with extra Docker options
pixi-docker build --no-cache --platform linux/amd64
```

### Run Docker Containers

```bash
# Run with automatic port mapping and naming
pixi-docker run

# Run specific environment
pixi-docker run -e dev

# Interactive shell access
pixi-docker run -it /bin/bash

# Run with custom Docker arguments
pixi-docker run --rm --name my-container

# Alternative syntax with -- separator (optional)
pixi-docker run -- --rm --name my-container

# Run with custom tag
pixi-docker run -t my-custom-name:v1.0
```

## Configuration Explained

### pixi.toml

- Defines the Pixi project with dependencies
- Sets up tasks for build, serve, and test
- Configures environments (prod, dev, test)

### pixi_docker.toml  

- Docker-specific configuration
- Different settings per environment
- Multi-stage build configuration
- Port and file copy settings

## Generated Dockerfiles

The plugin generates optimized multi-stage Dockerfiles:

1. **Build stage**: Installs dependencies and builds the project
2. **Production stage**: Minimal runtime image with only necessary files
3. **Shell-hook activation**: No pixi binary needed in final image

## Testing the Application

After building the Docker image:

```bash
# Run the container
docker run -p 8000:8000 my-pixi-app:1.0.0

# Visit http://localhost:8000 to see the demo page
```

## Environment Differences

- **prod**: Multi-stage build, minimal files, port 8000
- **dev**: Single-stage build, includes tests, ports 8000+3000  
- **test**: Optimized for testing, runs test suite

This example demonstrates a complete workflow from pixi project to production-ready Docker container.