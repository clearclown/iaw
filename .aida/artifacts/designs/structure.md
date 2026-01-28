# Phase 2: System Structure & Design

## Project Directory Structure

```
aether/
├── Cargo.toml                    # Root package manifest
├── README.md
├── LICENSE
├── .gitignore
├── aether.example.toml           # Example configuration
│
├── src/
│   ├── main.rs                   # CLI entry point
│   ├── lib.rs                    # Library exports
│   │
│   ├── cli/
│   │   ├── mod.rs                # CLI module
│   │   ├── commands.rs           # Command definitions (clap)
│   │   ├── workspace.rs          # Workspace subcommands
│   │   ├── run.rs                # Run command
│   │   └── status.rs             # Status command
│   │
│   ├── config/
│   │   ├── mod.rs
│   │   ├── schema.rs             # aether.toml schema
│   │   ├── loader.rs             # Config file loading
│   │   └── validation.rs         # Config validation
│   │
│   ├── jj/
│   │   ├── mod.rs
│   │   ├── delegation.rs         # jj command delegation
│   │   └── parser.rs             # jj output parsing
│   │
│   ├── provisioner/
│   │   ├── mod.rs
│   │   ├── manager.rs            # Main provisioner logic
│   │   ├── port_allocator.rs    # Dynamic port allocation
│   │   ├── context_injector.rs  # .env file generation
│   │   └── state.rs              # Workspace state management
│   │
│   ├── backend/
│   │   ├── mod.rs
│   │   ├── trait.rs              # Backend trait definition
│   │   ├── docker.rs             # Local Docker backend
│   │   ├── ssh_docker.rs         # Remote Docker via SSH
│   │   └── kubernetes.rs         # Future K8s backend
│   │
│   ├── output/
│   │   ├── mod.rs
│   │   ├── json.rs               # JSON output formatting
│   │   └── human.rs              # Human-readable output
│   │
│   └── error.rs                  # Error types
│
├── tests/
│   ├── integration_test.rs
│   ├── workspace_lifecycle.rs
│   └── port_allocation.rs
│
└── examples/
    ├── basic_workspace.rs
    └── multi_service.rs
```

## Core Data Schemas

### 1. Configuration Schema (aether.toml)

```rust
// config/schema.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AetherConfig {
    pub backend: BackendConfig,
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,
    #[serde(default)]
    pub injection: Option<InjectionConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    Docker {
        #[serde(default)]
        socket: Option<String>, // Optional custom socket path
    },
    Ssh {
        host: String,
        user: String,
        #[serde(default)]
        port: u16, // Default: 22
        #[serde(default)]
        key_path: Option<String>,
    },
    Kubernetes {
        context: String,
        #[serde(default)]
        namespace_prefix: String, // Default: "aether"
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
    pub image: String,
    #[serde(default)]
    pub ports: Vec<String>, // e.g., ["5432", "5433:5433"]
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<String>, // e.g., ["/host:/container"]
    #[serde(default)]
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InjectionConfig {
    pub file: String, // e.g., ".env"
    pub template: String, // Template with {{ placeholders }}
}
```

### 2. Workspace State Schema

```rust
// provisioner/state.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkspaceRegistry {
    pub version: String, // Schema version
    pub workspaces: HashMap<String, WorkspaceState>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkspaceState {
    pub name: String,
    pub path: String, // Absolute path
    pub namespace: String, // Unique identifier
    pub backend_type: String,
    pub created_at: String, // ISO 8601 timestamp
    pub resources: Vec<ResourceState>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResourceState {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>, // internal -> external
    pub status: ResourceStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ResourceStatus {
    Running,
    Stopped,
    Failed,
}
```

### 3. JSON Output Schema

```rust
// output/json.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct AjjOutput {
    pub status: OutputStatus,
    pub operation: String, // e.g., "workspace_add"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStatus {
    Success,
    Error,
    Partial, // Some operations succeeded
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub root: String, // Absolute path
    pub backend: String,
    pub namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<HashMap<String, ResourceInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub container_id: String,
    pub image: String,
    pub internal_port: u16,
    pub external_port: u16,
    pub host: String, // "127.0.0.1" or remote host
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String, // Machine-readable error code
    pub message: String, // Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
```

## CLI Interface Contracts

### Command Structure

```rust
// cli/commands.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ajj")]
#[command(about = "Aether - Infrastructure as Workspace", long_about = None)]
pub struct Cli {
    /// Output format (human, json)
    #[arg(short, long, default_value = "human", global = true)]
    pub output: OutputFormat,

    /// Configuration file path
    #[arg(short, long, default_value = "aether.toml", global = true)]
    pub config: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Workspace management
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Run command with environment loaded
    Run {
        /// Command to execute
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Show status of workspace and resources
    Status {
        /// Show JSON output
        #[arg(long)]
        json: bool,
    },

    /// List all workspaces
    List {
        /// Show JSON output
        #[arg(long)]
        json: bool,
    },

    /// Cleanup orphaned resources
    Cleanup {
        /// Actually perform cleanup (default is dry-run)
        #[arg(long)]
        force: bool,
    },

    // Passthrough for all other jj commands
    #[command(external_subcommand)]
    Jj(Vec<String>),
}

#[derive(Subcommand)]
pub enum WorkspaceAction {
    /// Create new workspace with infrastructure
    Add {
        /// Destination path
        destination: String,

        /// Revision to check out
        #[arg(short, long)]
        revision: Option<String>,
    },

    /// Remove workspace and cleanup infrastructure
    Forget {
        /// Workspace name or path
        workspace: String,
    },

    /// List workspaces
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}
```

### CLI Behavior Specification

#### 1. `ajj workspace add <destination>`

**Input:**
```bash
ajj workspace add ../feature-auth
ajj workspace add ../fix-login --revision abc123
ajj --output json workspace add ../feature-api
```

**Behavior:**
1. Validate `aether.toml` exists and is valid
2. Delegate to `jj workspace add <destination> [--revision REV]`
3. If jj succeeds:
   - Generate unique namespace: `aether-<repo_id>-<workspace_name>`
   - Call backend to spawn containers
   - Allocate dynamic ports for each service
   - Generate `.env` file from template
   - Store workspace state
4. Output result (JSON or human-readable)

**Success Output (JSON):**
```json
{
  "status": "success",
  "operation": "workspace_add",
  "workspace": {
    "name": "feature-auth",
    "root": "/home/user/project/feature-auth",
    "backend": "docker",
    "namespace": "aether-a4f3b-feature-auth",
    "resources": {
      "postgres": {
        "container_id": "8a7f2bc3",
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

**Error Output (JSON):**
```json
{
  "status": "error",
  "operation": "workspace_add",
  "workspace": null,
  "errors": [
    {
      "code": "JJ_FAILED",
      "message": "Jujutsu command failed: workspace already exists",
      "details": {
        "command": "jj workspace add ../feature-auth",
        "exit_code": 1
      }
    }
  ]
}
```

#### 2. `ajj workspace forget <workspace>`

**Input:**
```bash
ajj workspace forget feature-auth
ajj --output json workspace forget ../old-branch
```

**Behavior:**
1. Resolve workspace name/path to canonical name
2. Load workspace state from registry
3. Call backend to kill containers
4. Delegate to `jj workspace forget <workspace>`
5. Remove workspace from state registry
6. Output result

**Success Output (JSON):**
```json
{
  "status": "success",
  "operation": "workspace_forget",
  "workspace": {
    "name": "feature-auth",
    "root": null,
    "backend": "docker",
    "namespace": "aether-a4f3b-feature-auth",
    "resources": null
  },
  "errors": []
}
```

#### 3. `ajj run -- <command>`

**Input:**
```bash
ajj run -- cargo test
ajj run -- python manage.py migrate
```

**Behavior:**
1. Detect current workspace (via jj or file system)
2. Load `.env` file from workspace root
3. Inject environment variables
4. Execute command with inherited stdin/stdout/stderr
5. Return command exit code

**Output:**
- Transparent passthrough of command output
- Exit code matches executed command

#### 4. `ajj status`

**Input:**
```bash
ajj status
ajj status --json
```

**Behavior:**
1. Run `jj status` and capture output
2. Load current workspace state
3. Query backend for resource status
4. Combine and output

**Output (Human):**
```
Workspace: feature-auth
Backend: docker (local)
Namespace: aether-a4f3b-feature-auth

Resources:
  postgres (postgres:15)
    Container: 8a7f2bc3 [running]
    Ports: 5432 -> 32891

Jujutsu Status:
  Working copy: abc123def456
  Parent commit: main
  ...
```

**Output (JSON):**
```json
{
  "status": "success",
  "operation": "status",
  "workspace": {
    "name": "feature-auth",
    "root": "/home/user/project/feature-auth",
    "backend": "docker",
    "namespace": "aether-a4f3b-feature-auth",
    "resources": {
      "postgres": {
        "container_id": "8a7f2bc3",
        "image": "postgres:15",
        "internal_port": 5432,
        "external_port": 32891,
        "host": "127.0.0.1"
      }
    }
  },
  "errors": []
}
```

## Backend Trait Definition

```rust
// backend/trait.rs
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Backend: Send + Sync {
    /// Spawn containers for a workspace
    async fn provision(
        &self,
        namespace: &str,
        services: &HashMap<String, ServiceSpec>,
    ) -> Result<Vec<ResourceHandle>, BackendError>;

    /// Destroy all resources in a namespace
    async fn deprovision(&self, namespace: &str) -> Result<(), BackendError>;

    /// Query status of resources in a namespace
    async fn status(&self, namespace: &str) -> Result<Vec<ResourceStatus>, BackendError>;

    /// Get backend type identifier
    fn backend_type(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ServiceSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<u16>, // Internal ports
    pub env: HashMap<String, String>,
    pub volumes: Vec<VolumeMount>,
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct ResourceHandle {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>, // internal -> external
}

#[derive(Debug, Clone)]
pub struct ResourceStatus {
    pub service_name: String,
    pub container_id: String,
    pub status: ContainerStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Container spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Port allocation failed: {0}")]
    PortAllocationFailed(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Backend error: {0}")]
    Other(String),
}
```

## Port Allocation Strategy

```rust
// provisioner/port_allocator.rs
use std::collections::HashSet;
use std::net::TcpListener;

pub struct PortAllocator {
    range: (u16, u16), // (start, end)
    reserved: HashSet<u16>,
}

impl PortAllocator {
    pub fn new() -> Self {
        Self {
            range: (32768, 65535), // Ephemeral port range
            reserved: HashSet::new(),
        }
    }

    /// Allocate N random available ports
    pub fn allocate(&mut self, count: usize) -> Result<Vec<u16>, String> {
        let mut allocated = Vec::new();
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 1000;

        while allocated.len() < count && attempts < MAX_ATTEMPTS {
            let port = self.find_available_port()?;
            if !self.reserved.contains(&port) {
                self.reserved.insert(port);
                allocated.push(port);
            }
            attempts += 1;
        }

        if allocated.len() < count {
            Err(format!("Could not allocate {} ports", count))
        } else {
            Ok(allocated)
        }
    }

    /// Release ports back to the pool
    pub fn release(&mut self, ports: &[u16]) {
        for port in ports {
            self.reserved.remove(port);
        }
    }

    fn find_available_port(&self) -> Result<u16, String> {
        // Try to bind to port 0, OS will assign available port
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Port binding failed: {}", e))?;
        let port = listener.local_addr()
            .map_err(|e| format!("Could not get local addr: {}", e))?
            .port();
        Ok(port)
    }
}
```

## Context Injection

```rust
// provisioner/context_injector.rs
use handlebars::Handlebars;
use serde_json::json;
use std::collections::HashMap;

pub struct ContextInjector {
    handlebars: Handlebars<'static>,
}

impl ContextInjector {
    pub fn new() -> Self {
        Self {
            handlebars: Handlebars::new(),
        }
    }

    pub fn render(
        &self,
        template: &str,
        resources: &HashMap<String, ResourceHandle>,
    ) -> Result<String, String> {
        // Build context object
        let mut services = serde_json::Map::new();

        for (name, resource) in resources {
            let mut service_data = serde_json::Map::new();
            let mut ports = serde_json::Map::new();

            for (internal, external) in &resource.port_mappings {
                ports.insert(
                    internal.to_string(),
                    json!(external),
                );
            }

            service_data.insert("ports".to_string(), json!(ports));
            service_data.insert("container_id".to_string(), json!(resource.container_id));
            services.insert(name.clone(), json!(service_data));
        }

        let context = json!({
            "services": services
        });

        self.handlebars
            .render_template(template, &context)
            .map_err(|e| format!("Template render failed: {}", e))
    }
}
```

## Error Handling Strategy

```rust
// error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AetherError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Jujutsu error: {message}")]
    Jj {
        message: String,
        exit_code: i32,
    },

    #[error("Backend error: {0}")]
    Backend(#[from] crate::backend::BackendError),

    #[error("Port allocation error: {0}")]
    PortAllocation(String),

    #[error("Context injection error: {0}")]
    ContextInjection(String),

    #[error("State management error: {0}")]
    State(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AetherError>;
```

## Testing Strategy

### Unit Tests
- Port allocator: Verify no collisions in concurrent allocation
- Context injector: Template rendering edge cases
- Config parser: Valid and invalid TOML parsing

### Integration Tests
- Full workspace lifecycle (add → status → forget)
- Multi-service provisioning
- Port mapping correctness
- Cleanup reliability (no orphaned containers)

### Test Fixtures
```rust
// tests/fixtures/mod.rs
pub fn mock_aether_toml() -> String {
    r#"
[backend]
type = "docker"

[services.postgres]
image = "postgres:15-alpine"
ports = ["5432"]
env = { POSTGRES_PASSWORD = "test" }

[injection]
file = ".env"
template = "DATABASE_URL=postgres://postgres:test@localhost:{{ services.postgres.ports.5432 }}/db"
    "#.to_string()
}
```

## Development Roadmap

### MVP (Phase 1)
- ✅ CLI structure with clap
- ✅ Config loading (aether.toml)
- ✅ JJ delegation layer
- ✅ Local Docker backend
- ✅ Port allocator
- ✅ Context injector
- ✅ JSON output
- ✅ `workspace add/forget` commands
- ✅ `run` command

### Phase 2 (Remote)
- ✅ SSH Docker backend
- ✅ Port forwarding/tunneling
- ✅ Remote state sync

### Phase 3 (Advanced)
- ✅ Kubernetes backend
- ✅ MCP server implementation
- ✅ Workspace templates
- ✅ Multi-backend orchestration
