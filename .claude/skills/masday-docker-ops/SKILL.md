---
name: masday-docker-ops
description: >
  Docker container management for building images, running containers, and monitoring
  container state. Use when the user says "build docker", "run container", "docker status",
  "container management", or "docker operations".
allowed-tools:
  - docker.build
  - docker.run
  - docker.ps
  - filesystem.read
  - filesystem.list
  - memory.store
---

# Masday Docker Ops

Build, run, and manage Docker containers.

## Steps

1. **Check running containers**
   - Call `docker.ps` with `all: true` to see all containers (running and stopped)
   - Identify any containers relevant to the current task

2. **Read Dockerfile**
   - Call `filesystem.read` on the Dockerfile to review the build configuration
   - Check for: base image version, exposed ports, environment variables, build stages
   - Verify no secrets or credentials are hardcoded in the Dockerfile

3. **Build image**
   - Call `docker.build` with:
     - `tag`: descriptive tag (e.g., `masday-workflow:latest`, `masday-workflow:v1.2.0`)
     - `context`: build context directory (default: `.`)
     - `dockerfile`: path to Dockerfile (default: `Dockerfile`)
   - Monitor build output for errors or warnings

4. **Run container**
   - Call `docker.run` with:
     - `image`: the image tag just built or specified
     - `ports`: host-to-container port mappings (e.g., `[{"host": 3000, "container": 3000}]`)
     - `env`: environment variables (use `.env` file references, never hardcoded secrets)
     - `detach: true` for background services
     - `name`: descriptive container name
   - Example:
     ```
     docker.run({
       image: "masday-workflow:latest",
       ports: [{ host: 3000, container: 3000 }],
       env: [{ key: "NODE_ENV", value: "production" }],
       detach: true,
       name: "masday-api"
     })
     ```

5. **Verify container is running**
   - Call `docker.ps` to confirm the container is in "running" state
   - Check the port mappings are correct
   - If the container exited immediately, check logs for errors

6. **Store container details**
   - Call `memory.store` with `memory_type: "artifact"`:
     - Container name, image tag, port mappings, environment variables
     - Container ID and status

7. **Report**
   ```
   Docker Operations:
   - Built: masday-workflow:latest (2.3s)
   - Running: masday-api (container_id: abc123)
   - Ports: 3000 -> 3000
   - Status: healthy
   ```

## Never

- Never expose sensitive ports to 0.0.0.0 without user confirmation
- Never hardcode secrets in Dockerfiles or environment variables
- Never run containers in detached mode without storing their details for cleanup
- Never ignore build warnings -- they often indicate security or compatibility issues
