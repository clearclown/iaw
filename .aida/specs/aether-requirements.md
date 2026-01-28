# Aether (ajj) - Comprehensive Requirements Specification

## Document Information
- **Project**: Aether - Infrastructure as Workspace
- **Binary Name**: ajj
- **Version**: 1.0.0-MVP
- **Date**: 2026-01-28
- **Status**: Approved for Implementation

## 1. Executive Summary

Aether (ajj) is a Rust-based CLI tool that implements the "Infrastructure as Workspace" (IaW) paradigm by wrapping Jujutsu (jj) version control system and synchronizing workspace lifecycle with containerized infrastructure. Designed for AI-first parallel development, Aether eliminates port conflicts, resource contention, and environment drift by treating infrastructure as an ephemeral, workspace-bound resource.

### Key Innovation
Unlike traditional development where environments are singleton resources shared across branches, Aether creates isolated container environments for each VCS workspace, enabling true parallel development without conflicts.

## 2. Functional Requirements

### FR-001: Workspace Creation with Infrastructure Provisioning

**Priority**: P0 (Critical)

**User Story**: As an AI agent, I need to create a new workspace with dedicated infrastructure so that I can work in complete isolation from other parallel tasks.

**Command**: `ajj workspace add <destination> [--revision REV]`

**Behavior**:
1. Validate that `aether.toml` exists and is syntactically correct
2. Delegate to `jj workspace add <destination> [--revision REV]` to create VCS workspace
3. If jj command fails, abort with error (no infrastructure provisioning)
4. If jj succeeds:
   a. Generate unique namespace: `aether-<repo_hash>-<workspace_name>`
   b. Parse service definitions from `aether.toml`
   c. Allocate N dynamic ports (where N = total container ports needed)
   d. Invoke backend to spawn containers with port mappings
   e. Wait for containers to be in "running" state
   f. Generate `.env` file from template with actual port values
   g. Store workspace metadata in state registry
5. Output result in requested format (JSON or human-readable)

**Input Validation**:
- `destination` must be valid path (relative or absolute)
- `destination` must not already exist as workspace
- Optional `--revision` must be valid jj revision identifier

**Output (JSON)**:
```json
{
  "status": "success",
  "operation": "workspace_add",
  "workspace": {
    "name": "feature-auth",
    "root": "/absolute/path/to/workspace",
    "backend": "docker",
    "namespace": "aether-a4f3b-feature-auth",
    "resources": {
      "postgres": {
        "container_id": "8a7f2bc3def4",
        "image": "postgres:15",
        "internal_port": 5432,
        "external_port": 32891,
        "host": "127.0.0.1",
        "connection_string": "postgres://postgres:password@127.0.0.1:32891/mydb"
      }
    },
    "env_file": ".env"
  },
  "errors": []
}
```

**Error Cases**:
- `JJ_NOT_FOUND`: jj binary not in PATH
- `JJ_FAILED`: jj workspace add command failed
- `CONFIG_NOT_FOUND`: aether.toml not found
- `CONFIG_INVALID`: aether.toml syntax error
- `PORT_ALLOCATION_FAILED`: No available ports
- `BACKEND_SPAWN_FAILED`: Container failed to start
- `CONTEXT_INJECTION_FAILED`: .env file write failed

**Performance Target**: Total overhead < 1 second excluding container startup time

---

### FR-002: Workspace Cleanup with Infrastructure Teardown

**Priority**: P0 (Critical)

**User Story**: As an AI agent, I need to remove a workspace and automatically cleanup all associated infrastructure so that I don't leave orphaned containers consuming resources.

**Command**: `ajj workspace forget <workspace>`

**Behavior**:
1. Resolve `<workspace>` argument to workspace name:
   - If contains `/`, treat as path and convert to workspace name
   - Otherwise, treat as workspace name directly
2. Load workspace metadata from state registry
3. If workspace not found in registry, warn but continue with jj operation
4. Invoke backend to destroy all containers in workspace namespace
5. Delete workspace entry from state registry
6. Delegate to `jj workspace forget <workspace>`
7. Output result

**Input Validation**:
- Workspace must exist (either in state registry or as jj workspace)

**Output (JSON)**:
```json
{
  "status": "success",
  "operation": "workspace_forget",
  "workspace": {
    "name": "feature-auth",
    "namespace": "aether-a4f3b-feature-auth",
    "resources_destroyed": 2
  },
  "errors": []
}
```

**Error Cases**:
- `WORKSPACE_NOT_FOUND`: Workspace doesn't exist
- `BACKEND_DEPROVISION_FAILED`: Container cleanup failed
- `JJ_FAILED`: jj workspace forget failed

**Guarantee**: Best-effort cleanup. If backend destroy fails, record error but don't block jj operation.

---

### FR-003: Dynamic Port Allocation

**Priority**: P0 (Critical)

**User Story**: As a developer running multiple workspaces, I need automatic port allocation so that I never encounter "Address already in use" errors.

**Mechanism**:
1. For each service in `aether.toml` with `ports = [...]`, allocate one host port per container port
2. Use ephemeral port range (32768-65535) by default
3. Verify port is available by attempting bind
4. Store allocation in workspace state for later reference
5. Pass mappings to backend as `container_port:host_port` pairs

**Concurrency Safety**:
- Must be safe for concurrent `workspace add` operations
- Use file-based locking on state registry
- PortAllocator must be wrapped in Mutex

**Allocation Strategy**:
- Prefer OS-assigned ports (bind to `:0` then query assigned port)
- If specific port requested in format `"5432:5432"`, use that (error if unavailable)

**Port Release**:
- Ports are released when workspace is forgotten
- Orphaned port allocations can be reclaimed via `ajj cleanup`

---

### FR-004: Context Injection via Template Rendering

**Priority**: P0 (Critical)

**User Story**: As an application running in a workspace, I need connection strings with correct ports injected into my environment so that I can connect to workspace-specific services.

**Mechanism**:
1. Read `[injection]` section from `aether.toml`
2. Parse `template` string as Handlebars template
3. Build context object with structure:
   ```json
   {
     "services": {
       "<service_name>": {
         "ports": {
           "<internal_port>": <external_port>
         },
         "container_id": "..."
       }
     }
   }
   ```
4. Render template with context
5. Write result to `<workspace_root>/<injection.file>` (e.g., `.env`)

**Template Syntax**:
```
DATABASE_URL=postgres://user:pass@localhost:{{ services.postgres.ports.5432 }}/db
REDIS_URL=redis://localhost:{{ services.redis.ports.6379 }}
```

**Error Handling**:
- Invalid template syntax: Abort workspace creation
- Missing service reference: Abort workspace creation
- File write failure: Abort workspace creation with rollback

**Future Enhancement**: Add Handlebars helpers (random_string, base64)

---

### FR-005: Transparent Command Execution

**Priority**: P0 (Critical)

**User Story**: As an AI agent, I need to run tests and scripts with workspace-specific environment variables loaded automatically.

**Command**: `ajj run -- <command> [args...]`

**Behavior**:
1. Detect current workspace (via jj or filesystem inspection)
2. If not in a workspace, error: "Not in an Aether-managed workspace"
3. Load `.env` file from workspace root (if exists)
4. Parse `.env` into key=value pairs
5. Merge with current process environment (system env takes precedence)
6. Spawn `<command>` as subprocess with merged environment
7. Stream stdout/stderr directly to terminal (unbuffered)
8. Return subprocess exit code as ajj exit code

**Input Validation**:
- Command must be provided (cannot be empty)
- Command must be executable or in PATH

**Special Behavior**:
- Preserve signals (SIGINT, SIGTERM) to subprocess
- Preserve stdin passthrough for interactive commands
- No JSON output mode (this is a transparent proxy)

---

### FR-006: Backend Abstraction Layer

**Priority**: P0 (Critical)

**User Story**: As a system architect, I need a pluggable backend system so that Aether can support local Docker, remote Docker, and Kubernetes without changing core logic.

**Design**: Rust trait-based abstraction

**Trait Interface**:
```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn provision(&self, namespace: &str, services: &HashMap<String, ServiceSpec>) -> Result<Vec<ResourceHandle>>;
    async fn deprovision(&self, namespace: &str) -> Result<()>;
    async fn status(&self, namespace: &str) -> Result<Vec<ResourceStatus>>;
    fn backend_type(&self) -> &'static str;
}
```

**Supported Backends (MVP)**:
1. **Local Docker**: Uses `bollard` crate to communicate with local Docker daemon
2. **Remote Docker via SSH**: Executes Docker CLI commands over SSH connection

**Supported Backends (Future)**:
3. **Kubernetes**: Uses `kube` crate to deploy pods in namespaced environment

**Backend Selection**:
- Determined by `[backend]` section in `aether.toml`
- Backend instance created at CLI startup
- Single backend per ajj invocation

---

### FR-007: Configuration File Schema

**Priority**: P0 (Critical)

**File Format**: TOML (Tom's Obvious, Minimal Language)

**Location**: `aether.toml` in repository root

**Schema**:
```toml
[backend]
type = "docker" | "ssh" | "kubernetes"

# For SSH backend
host = "devbox.company.com"
user = "deploy"
port = 22 # optional, default 22
key_path = "~/.ssh/id_rsa" # optional

[services.<name>]
image = "docker/image:tag"
ports = ["5432", "8080:8080"] # Container ports or explicit mappings
env = { KEY = "value", ... } # Environment variables
volumes = ["/host:/container", ...] # Volume mounts
command = ["override", "cmd"] # Optional command override

[injection]
file = ".env" # Relative to workspace root
template = "..."
```

**Validation Rules**:
- `backend.type` is required and must be valid enum value
- Each service must have at least `image` field
- Port specifications must be valid integers or colon-separated pairs
- Template must reference only defined services

**Discovery Algorithm**:
1. Check for `--config <path>` CLI flag
2. Otherwise, start from current directory
3. Check for `aether.toml` in current directory
4. Walk up parent directories until `.jj` directory found (repo root)
5. If not found by repo root, error: "aether.toml not found"

---

### FR-008: Workspace State Management

**Priority**: P0 (Critical)

**Purpose**: Track active workspaces and their associated containers for cleanup and status queries

**Storage Location**: `.aether/state.json` in repository root

**Schema**:
```json
{
  "version": "1.0",
  "workspaces": {
    "feature-auth": {
      "name": "feature-auth",
      "path": "/absolute/path",
      "namespace": "aether-a4f3b-feature-auth",
      "backend_type": "docker",
      "created_at": "2026-01-28T10:00:00Z",
      "config_hash": "sha256:abc123...",
      "resources": [
        {
          "service_name": "postgres",
          "container_id": "8a7f2bc3",
          "image": "postgres:15",
          "port_mappings": { "5432": 32891 },
          "status": "running"
        }
      ]
    }
  }
}
```

**Operations**:
- `register_workspace()`: Add new workspace entry
- `unregister_workspace()`: Remove workspace entry
- `get_workspace()`: Retrieve workspace metadata
- `list_workspaces()`: Get all workspaces

**Concurrency**: File-based locking using `fs2` crate's `FileLock`

**Atomicity**: Write to `.state.json.tmp` then rename for atomic updates

---

### FR-009: Orphan Container Cleanup

**Priority**: P1 (High)

**User Story**: As a developer, I need a way to cleanup containers left behind by crashed ajj processes.

**Command**: `ajj cleanup [--force]`

**Behavior (default = dry-run)**:
1. Query backend for all containers with label `aether.managed=true`
2. Load workspace state registry
3. Identify containers not in registry (orphans)
4. Display list of orphans with details
5. If `--force` flag present, destroy orphaned containers
6. Update state registry if needed

**Container Labels** (for identification):
- `aether.managed=true`
- `aether.workspace=<workspace_name>`
- `aether.namespace=<namespace>`
- `aether.service=<service_name>`

---

### FR-010: Status Reporting

**Priority**: P1 (High)

**User Story**: As a user, I need to inspect the current workspace's infrastructure status.

**Command**: `ajj status [--json]`

**Behavior**:
1. Detect current workspace
2. Run `jj status` and capture output
3. Load workspace metadata from state
4. Query backend for current resource status
5. Combine jj status and infrastructure status
6. Output in requested format

**Output** includes:
- Workspace name and path
- Backend type
- List of containers with status (running/stopped/failed)
- Port mappings
- JJ working copy status

---

### FR-011: Workspace Listing

**Priority**: P2 (Medium)

**Command**: `ajj list [--json]`

**Behavior**:
1. Load state registry
2. For each workspace, query backend for resource status
3. Output table or JSON array of workspaces

**Output Columns**:
- Workspace name
- Path
- Backend
- Resource count
- Status (healthy/degraded/failed)

## 3. Non-Functional Requirements

### NFR-001: Performance
- **Requirement**: CLI overhead < 1 second for all commands (excluding I/O)
- **Rationale**: Maintain jj's native speed, avoid frustrating AI agents
- **Measurement**: Unit test benchmarks for core operations

### NFR-002: AI-First UX
- **Requirement**: All commands support `--output json` for structured output
- **Rationale**: Enable AI agents to programmatically parse results
- **Design Principle**: Favor arguments over interactive prompts

### NFR-003: Determinism
- **Requirement**: Same configuration produces byte-identical environments
- **Rationale**: Eliminate environmental variance as a debugging variable
- **Exception**: Port numbers will vary (unavoidable)

### NFR-004: Resource Isolation
- **Requirement**: Zero shared state between workspace environments
- **Mechanism**: Unique Docker networks, volumes, and containers per workspace

### NFR-005: Reliability
- **Requirement**: 100% cleanup rate for managed containers
- **Mechanism**: Transactional state updates, idempotent cleanup operations

### NFR-006: Observability
- **Requirement**: All errors include machine-readable error codes
- **Format**: JSON errors have `code`, `message`, `details` fields

## 4. System Constraints

### Technical Constraints
- **Language**: Rust 1.75+ (2021 edition)
- **Target Platforms**: Linux, macOS (Windows future)
- **External Dependencies**:
  - Jujutsu (jj) >= 0.12.0 must be installed
  - Docker Engine >= 20.10 (for Docker backend)
  - SSH client (for remote backend)

### Operational Constraints
- State file size grows linearly with workspace count
- Port allocation limited by OS ephemeral port range (typically ~28,000 ports)
- Concurrent workspace creation limited by state file lock contention

## 5. Security Requirements

### SEC-001: Credential Management
- **Requirement**: Never log or output credentials in plain text
- **Mechanism**: Redact environment variables matching patterns (PASSWORD, SECRET, TOKEN)

### SEC-002: Command Injection Prevention
- **Requirement**: No shell interpolation of user input
- **Mechanism**: Use Rust `Command::arg()` for argument passing, never string concatenation

### SEC-003: SSH Key Protection
- **Requirement**: SSH private keys must have restricted permissions (0600)
- **Validation**: Error if key file is world-readable

## 6. Acceptance Criteria

The system is considered complete when:

1. An AI agent can execute the following workflow without errors:
   ```bash
   ajj workspace add ../task-1
   cd ../task-1
   ajj run -- cargo test
   cd ..
   ajj workspace forget task-1
   ```

2. Ten parallel workspace creations complete without port conflicts

3. All containers are cleaned up after `workspace forget` (verified by `docker ps`)

4. JSON output parses correctly with standard JSON parsers

5. Configuration errors produce actionable error messages

6. State file remains consistent under concurrent operations (verified by stress test)

## 7. Future Enhancements (Out of Scope for MVP)

- **Volume persistence**: Persist database state across workspace recreations
- **Kubernetes backend**: Deploy to K8s clusters
- **Health checks**: Wait for service readiness before returning
- **Resource limits**: Configure CPU/memory limits per service
- **MCP server**: Model Context Protocol integration for AI frameworks
- **Workspace templates**: Pre-configured service bundles
- **Remote tunneling**: SSH port forwarding for remote backends

## 8. Glossary

- **IaW**: Infrastructure as Workspace - paradigm linking VCS and containers
- **Workspace**: Jujutsu working copy at a specific file system location
- **Namespace**: Unique identifier for workspace's container environment
- **Backend**: Container runtime abstraction (Docker, SSH, K8s)
- **Service**: Containerized application defined in aether.toml
- **Resource**: Running container instance
- **Context Injection**: Writing runtime values to workspace environment files

---

**Specification Status**: ✅ APPROVED FOR IMPLEMENTATION
