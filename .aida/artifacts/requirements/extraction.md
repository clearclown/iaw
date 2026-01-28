# Phase 1: Requirements Extraction & Architecture

## Executive Summary

Aether (ajj) is a Rust CLI tool implementing "Infrastructure as Workspace" (IaW) - a paradigm that synchronizes version control workspace lifecycle with containerized infrastructure. Designed for AI-first development, it enables parallel development environments without resource conflicts.

## Core Requirements Extraction

### 1. Workspace Management
**FR-WS-001**: Workspace Creation with Infrastructure Provisioning
- **Input**: `ajj workspace add <destination>`
- **Behavior**:
  1. Delegate to `jj workspace add <destination>` to create VCS workspace
  2. Spawn isolated container environment with unique namespace/ID
  3. Dynamically allocate available host ports for container port mappings
  4. Write connection information to `.env` file in workspace root
  5. Return structured JSON output for AI agent consumption
- **Output**: JSON with workspace path, port mappings, connection strings

**FR-WS-002**: Workspace Cleanup
- **Input**: `ajj workspace forget <workspace>`
- **Behavior**:
  1. Delegate to `jj workspace forget <workspace>`
  2. Identify and kill all containers associated with workspace namespace
  3. Remove volumes, networks, and ephemeral resources
  4. Confirm complete cleanup
- **Output**: Success/failure status

### 2. Dynamic Port Mapping
**FR-PORT-001**: Automatic Port Allocation
- **Requirement**: Avoid "Address already in use" conflicts
- **Mechanism**:
  - Scan for available ports in ephemeral range (32768-65535)
  - Map container internal ports (e.g., 5432 for Postgres) to random available host ports
  - Store mapping metadata for context injection

**FR-PORT-002**: Port Mapping Persistence
- **Requirement**: Port allocations must persist across CLI invocations
- **Mechanism**: Store mappings in workspace metadata file or state database

### 3. Context Injection
**FR-CTX-001**: Environment Variable Injection
- **Input**: Template from `aether.toml` configuration
- **Behavior**:
  - Parse template strings with placeholders (e.g., `{{ services.postgres.ports.5432 }}`)
  - Substitute with actual runtime values
  - Write to `.env` file in workspace root
- **Output**: Generated `.env` file

**FR-CTX-002**: Structured Output for AI Agents
- **Requirement**: Emit machine-readable JSON to stdout
- **Schema**:
```json
{
  "status": "ready" | "error",
  "workspace_root": "/absolute/path",
  "backend": "docker" | "kubernetes" | "ssh",
  "resources": {
    "<service_name>": {
      "container_id": "8a7f2b...",
      "internal_port": 5432,
      "external_port": 32891,
      "host": "127.0.0.1",
      "connection_string": "postgres://..."
    }
  },
  "env_file": ".env",
  "errors": []
}
```

### 4. Transparent Command Execution
**FR-EXEC-001**: Run Commands with Environment
- **Input**: `ajj run -- <command>`
- **Behavior**:
  1. Load `.env` from current workspace
  2. Inject environment variables into command execution context
  3. Execute command transparently
  4. (Optional) Establish SSH tunnel if using remote backend
- **Output**: Command stdout/stderr passthrough

### 5. Backend Abstraction
**FR-BE-001**: Pluggable Backend Architecture
- **Supported Backends**:
  - **Local Docker** (MVP): Use local Docker daemon
  - **Remote Docker via SSH**: Execute Docker commands on remote host via SSH
  - **Kubernetes Namespace** (Future): Deploy pods in isolated namespace
- **Interface**: Trait-based abstraction for backend operations

**FR-BE-002**: Configuration-Driven Backend Selection
- **Input**: `aether.toml` with `[backend]` section
- **Example**:
```toml
[backend]
type = "docker"  # or "ssh", "kubernetes"
# SSH-specific config
host = "devbox.company.com"
user = "deploy"
```

### 6. Configuration Schema
**FR-CFG-001**: aether.toml Format
```toml
[backend]
type = "docker"

[services.postgres]
image = "postgres:15"
ports = ["5432"]
env = { POSTGRES_PASSWORD = "secret" }
volumes = ["/data:/var/lib/postgresql/data"]

[services.redis]
image = "redis:7"
ports = ["6379"]

[injection]
file = ".env"
template = """
DATABASE_URL=postgres://user:pass@localhost:{{ services.postgres.ports.5432 }}/db
REDIS_URL=redis://localhost:{{ services.redis.ports.6379 }}
"""
```

## High-Level Architecture

### Component Diagram
```
┌─────────────────────────────────────────────────────┐
│                   ajj CLI                            │
│  ┌──────────────┐  ┌─────────────────────────────┐  │
│  │   Command    │  │   JJ Delegation Layer       │  │
│  │   Parser     │──│   (subprocess execution)    │  │
│  └──────────────┘  └─────────────────────────────┘  │
│         │                                            │
│         ▼                                            │
│  ┌──────────────────────────────────────────────┐  │
│  │    Resource Provisioner                      │  │
│  │  ┌──────────────┐  ┌───────────────────┐    │  │
│  │  │  Port        │  │  Context          │    │  │
│  │  │  Allocator   │  │  Injector         │    │  │
│  │  └──────────────┘  └───────────────────┘    │  │
│  └──────────────────────────────────────────────┘  │
│         │                                            │
│         ▼                                            │
│  ┌──────────────────────────────────────────────┐  │
│  │    Backend Trait (Abstraction)               │  │
│  │  ┌────────┐  ┌────────┐  ┌──────────────┐   │  │
│  │  │ Docker │  │  SSH   │  │ Kubernetes   │   │  │
│  │  │ Impl   │  │ Docker │  │ (Future)     │   │  │
│  │  └────────┘  └────────┘  └──────────────┘   │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
           │                                │
           ▼                                ▼
    ┌─────────────┐                  ┌───────────┐
    │   Docker    │                  │  Remote   │
    │   Engine    │                  │  Docker   │
    └─────────────┘                  └───────────┘
```

### Sequence Diagram: Workspace Creation
```
AI Agent         ajj CLI          JJ Binary       Backend         File System
   │                │                 │               │                │
   │ workspace add  │                 │               │                │
   ├───────────────>│                 │               │                │
   │                │ workspace add   │               │                │
   │                ├────────────────>│               │                │
   │                │    success      │               │                │
   │                │<────────────────┤               │                │
   │                │                 │   spawn       │                │
   │                │                 │   containers  │                │
   │                ├─────────────────────────────────>│                │
   │                │                 │   {ports}     │                │
   │                │<─────────────────────────────────┤                │
   │                │                 │               │                │
   │                │                 │            write .env          │
   │                ├────────────────────────────────────────────────->│
   │    JSON info   │                 │               │                │
   │<───────────────┤                 │               │                │
```

## Non-Functional Requirements

### NFR-001: Performance
- **Requirement**: `workspace add` overhead < 1 second (excluding container startup)
- **Rationale**: Maintain jj's native speed, avoid frustrating AI agents

### NFR-002: AI-First UX
- **Requirement**: Structured JSON output for all operations
- **Rationale**: Enable AI agents to parse and act on results programmatically
- **Design Principle**: Argument-based control > interactive prompts

### NFR-003: Determinism
- **Requirement**: Same configuration produces identical environments
- **Rationale**: Eliminate AI debugging overhead caused by environmental variance

### NFR-004: Transparency
- **Requirement**: Developers interact via `localhost` regardless of backend location
- **Mechanism**: Port forwarding / SSH tunneling for remote backends

### NFR-005: Resource Isolation
- **Requirement**: Complete isolation between workspace environments
- **Mechanism**: Unique namespace/tag per workspace, no shared state

## Technology Stack

### Language & Tooling
- **Language**: Rust (2021 edition)
- **Binary Name**: `ajj`
- **CLI Framework**: `clap` v4 (derive API)
- **Error Handling**: `anyhow` for application errors, `thiserror` for library errors
- **Async Runtime**: `tokio` (for async I/O with containers)
- **Configuration**: `serde` + `toml` for `aether.toml` parsing
- **JSON Output**: `serde_json`
- **Process Execution**: `std::process::Command` for jj delegation

### External Dependencies
- **Jujutsu (jj)**: Must be installed and accessible in PATH
- **Docker**: For local backend
- **SSH Client**: For remote backend
- **kubectl**: For Kubernetes backend (future)

## Risk Analysis

### Risk 1: Port Allocation Races
- **Issue**: Multiple concurrent `workspace add` operations may allocate same port
- **Mitigation**: File-based locking or atomic state updates

### Risk 2: Orphaned Containers
- **Issue**: Crash during workspace creation leaves running containers
- **Mitigation**: Implement cleanup hook on startup, list orphaned containers

### Risk 3: jj API Instability
- **Issue**: Jujutsu is pre-1.0, CLI may change
- **Mitigation**: Version pin jj requirement, abstract jj interactions behind trait

### Risk 4: SSH Connection Reliability
- **Issue**: Remote backend requires stable SSH connection
- **Mitigation**: Implement retry logic, connection pooling

## Success Criteria

1. ✅ AI agent can create 10 parallel workspaces without conflicts
2. ✅ Port allocation is deterministic and conflict-free
3. ✅ `.env` injection works correctly with template substitution
4. ✅ Container cleanup is 100% reliable (no leaks)
5. ✅ CLI overhead adds < 1 second to jj operations
6. ✅ JSON output is parseable by standard JSON parsers
7. ✅ Remote Docker backend works identically to local backend

## Open Questions

1. **State Storage**: Where to store workspace→container mappings?
   - Option A: `.aether/state.json` in repo root
   - Option B: SQLite database in `~/.config/aether/`
   - **Decision**: Start with JSON file for MVP

2. **Namespace Format**: How to name containers/namespaces?
   - Proposal: `aether-<repo_hash>-<workspace_name>`
   - Must be DNS-safe for Kubernetes

3. **Port Range**: Use ephemeral range (32768-65535) or custom range?
   - **Decision**: Start with ephemeral, make configurable

4. **Error Recovery**: How to handle partial failures?
   - Example: Container spawned but .env injection failed
   - **Decision**: Implement rollback mechanism
