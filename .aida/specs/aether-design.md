# Aether (ajj) - Technical Design Specification

## Document Information
- **Project**: Aether - Infrastructure as Workspace
- **Version**: 1.0.0-MVP
- **Date**: 2026-01-28
- **Status**: Approved for Implementation

## 1. System Architecture

### 1.1 High-Level Architecture

```
┌────────────────────────────────────────────────────────────┐
│                     ajj CLI Process                         │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Command Parser (clap)                               │  │
│  │  - Parse args                                        │  │
│  │  - Validate inputs                                   │  │
│  │  - Route to handlers                                 │  │
│  └────────────┬─────────────────────────────────────────┘  │
│               │                                              │
│               ▼                                              │
│  ┌──────────────────────┐      ┌──────────────────────┐   │
│  │  JJ Delegation       │      │  Config Loader       │   │
│  │  - subprocess spawn  │      │  - TOML parsing      │   │
│  │  - output capture    │      │  - validation        │   │
│  └──────────────────────┘      └──────────────────────┘   │
│               │                           │                  │
│               ▼                           ▼                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Resource Provisioner                                │  │
│  │  ┌───────────────┐  ┌────────────────────────────┐  │  │
│  │  │ Port          │  │ Context Injector           │  │  │
│  │  │ Allocator     │  │ (Handlebars)               │  │  │
│  │  └───────────────┘  └────────────────────────────┘  │  │
│  │  ┌───────────────────────────────────────────────┐  │  │
│  │  │ State Manager (WorkspaceRegistry)             │  │  │
│  │  │ - JSON state file (.aether/state.json)        │  │  │
│  │  │ - File locking (fs2)                          │  │  │
│  │  └───────────────────────────────────────────────┘  │  │
│  └────────────┬─────────────────────────────────────────┘  │
│               │                                              │
│               ▼                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Backend Trait (async)                               │  │
│  │  ┌────────────┐  ┌────────────┐  ┌──────────────┐   │  │
│  │  │ Docker     │  │ SSH Docker │  │ Kubernetes   │   │  │
│  │  │ (bollard)  │  │ (ssh2)     │  │ (kube)       │   │  │
│  │  └────────────┘  └────────────┘  └──────────────┘   │  │
│  └────────────┬─────────────────────────────────────────┘  │
│               │                                              │
└───────────────┼──────────────────────────────────────────────┘
                │
                ▼
    ┌────────────────────────┐
    │  Container Runtime     │
    │  - Docker Engine       │
    │  - Remote Docker Host  │
    │  - K8s Cluster         │
    └────────────────────────┘
```

### 1.2 Component Interaction Flow

**Workspace Creation Sequence**:
```
User → CLI Parser → Config Loader → JJ Delegation
                                          ↓
                                    [jj creates workspace]
                                          ↓
                    Port Allocator ← Provisioner
                           ↓
                    Backend::provision() → Docker API
                           ↓
                    [containers spawn]
                           ↓
                    Context Injector → .env file
                           ↓
                    State Manager → .aether/state.json
                           ↓
                    JSON Formatter → stdout
```

## 2. Module Design

### 2.1 CLI Module (`src/cli/`)

**Responsibilities**:
- Parse command-line arguments using `clap`
- Route to appropriate command handlers
- Format and output results

**Key Files**:

```rust
// src/cli/commands.rs
#[derive(Parser)]
pub struct Cli {
    #[arg(short, long, default_value = "human", global = true)]
    pub output: OutputFormat,

    #[arg(short, long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Workspace { #[command(subcommand)] action: WorkspaceAction },
    Run { #[arg(last = true)] command: Vec<String> },
    Status { #[arg(long)] json: bool },
    List { #[arg(long)] json: bool },
    Cleanup { #[arg(long)] force: bool },
    #[command(external_subcommand)]
    Jj(Vec<String>),
}
```

**Design Decisions**:
- Use `#[command(external_subcommand)]` for jj passthrough
- Global flags apply to all subcommands
- `OutputFormat` enum controls human vs JSON output

### 2.2 Configuration Module (`src/config/`)

**Responsibilities**:
- Load and parse `aether.toml`
- Validate configuration schema
- Provide typed access to config values

**Key Structures**:

```rust
// src/config/schema.rs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AetherConfig {
    pub backend: BackendConfig,
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,
    pub injection: Option<InjectionConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    Docker { socket: Option<String> },
    Ssh { host: String, user: String, port: Option<u16>, key_path: Option<String> },
    Kubernetes { context: String, namespace_prefix: Option<String> },
}
```

**Discovery Algorithm**:
```rust
// src/config/loader.rs
pub fn find_config(start_path: &Path) -> Result<PathBuf> {
    let mut current = start_path;
    loop {
        let candidate = current.join("aether.toml");
        if candidate.exists() {
            return Ok(candidate);
        }

        // Stop at repo root (.jj directory)
        if current.join(".jj").is_dir() {
            break;
        }

        current = current.parent()
            .ok_or_else(|| AetherError::Config("aether.toml not found".into()))?;
    }
    Err(AetherError::Config("aether.toml not found in repo".into()))
}
```

### 2.3 JJ Delegation Module (`src/jj/`)

**Responsibilities**:
- Execute jj commands as subprocess
- Capture and parse jj output
- Handle jj errors

**Key Functions**:

```rust
// src/jj/delegation.rs
use std::process::{Command, Stdio};

pub struct JjCommand {
    args: Vec<String>,
}

impl JjCommand {
    pub fn workspace_add(destination: &str, revision: Option<&str>) -> Self {
        let mut args = vec!["workspace".to_string(), "add".to_string(), destination.to_string()];
        if let Some(rev) = revision {
            args.push("--revision".to_string());
            args.push(rev.to_string());
        }
        Self { args }
    }

    pub fn execute(&self) -> Result<JjOutput> {
        let output = Command::new("jj")
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| AetherError::Jj {
                message: format!("Failed to execute jj: {}", e),
                exit_code: -1,
            })?;

        if !output.status.success() {
            return Err(AetherError::Jj {
                message: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
            });
        }

        Ok(JjOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}
```

**Error Handling**:
- `jj` not in PATH → `AetherError::Jj` with actionable message
- Non-zero exit code → Capture stderr and propagate as error
- Command spawn failure → Distinguish from command failure

### 2.4 Provisioner Module (`src/provisioner/`)

**Responsibilities**:
- Orchestrate workspace lifecycle
- Allocate ports
- Inject context
- Manage state

#### 2.4.1 Port Allocator

```rust
// src/provisioner/port_allocator.rs
use std::sync::Mutex;
use std::net::TcpListener;

pub struct PortAllocator {
    inner: Mutex<PortAllocatorInner>,
}

struct PortAllocatorInner {
    range: (u16, u16),
    reserved: HashSet<u16>,
}

impl PortAllocator {
    pub fn allocate(&self, count: usize) -> Result<Vec<u16>> {
        let mut inner = self.inner.lock().unwrap();
        let mut allocated = Vec::new();

        for _ in 0..count {
            let port = Self::find_free_port()?;
            inner.reserved.insert(port);
            allocated.push(port);
        }

        Ok(allocated)
    }

    fn find_free_port() -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        Ok(port)
    }

    pub fn release(&self, ports: &[u16]) {
        let mut inner = self.inner.lock().unwrap();
        for port in ports {
            inner.reserved.remove(port);
        }
    }
}
```

**Thread Safety**: Mutex ensures single allocator instance can be safely shared

**OS Integration**: Bind to port 0 delegates allocation to OS kernel

#### 2.4.2 Context Injector

```rust
// src/provisioner/context_injector.rs
use handlebars::Handlebars;
use serde_json::json;

pub struct ContextInjector {
    handlebars: Handlebars<'static>,
}

impl ContextInjector {
    pub fn render(
        &self,
        template: &str,
        resources: &HashMap<String, ResourceHandle>,
    ) -> Result<String> {
        let mut services = serde_json::Map::new();

        for (name, resource) in resources {
            let mut ports_map = serde_json::Map::new();
            for (internal, external) in &resource.port_mappings {
                ports_map.insert(internal.to_string(), json!(external));
            }

            services.insert(name.clone(), json!({
                "ports": ports_map,
                "container_id": resource.container_id,
            }));
        }

        let context = json!({ "services": services });

        self.handlebars.render_template(template, &context)
            .map_err(|e| AetherError::ContextInjection(e.to_string()))
    }
}
```

**Template Example**:
```
DATABASE_URL=postgres://user:pass@localhost:{{ services.postgres.ports.5432 }}/db
```

**Error Cases**:
- Invalid template syntax → Parse error with line/column
- Missing service reference → Error listing available services

#### 2.4.3 State Manager

```rust
// src/provisioner/state.rs
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

pub struct StateManager {
    state_file: PathBuf,
}

impl StateManager {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            state_file: repo_root.join(".aether/state.json"),
        }
    }

    pub fn register_workspace(&self, workspace: WorkspaceState) -> Result<()> {
        let lock_file = self.acquire_lock()?;

        let mut registry = self.load_registry()?;
        registry.workspaces.insert(workspace.name.clone(), workspace);

        self.atomic_write(&registry)?;
        drop(lock_file); // Release lock
        Ok(())
    }

    fn acquire_lock(&self) -> Result<File> {
        let lock_path = self.state_file.with_extension("lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(lock_path)?;
        file.lock_exclusive()?;
        Ok(file)
    }

    fn atomic_write(&self, registry: &WorkspaceRegistry) -> Result<()> {
        let tmp_path = self.state_file.with_extension("tmp");
        let json = serde_json::to_string_pretty(registry)?;
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(tmp_path, &self.state_file)?;
        Ok(())
    }
}
```

**Concurrency**: File locking prevents concurrent modifications

**Atomicity**: Write-then-rename ensures no partial state

**Recovery**: Lock file auto-released on process exit

### 2.5 Backend Module (`src/backend/`)

**Responsibilities**:
- Abstract container runtime operations
- Implement backend-specific logic

#### 2.5.1 Backend Trait

```rust
// src/backend/trait.rs
use async_trait::async_trait;

#[async_trait]
pub trait Backend: Send + Sync {
    async fn provision(
        &self,
        namespace: &str,
        services: &HashMap<String, ServiceSpec>,
    ) -> Result<Vec<ResourceHandle>>;

    async fn deprovision(&self, namespace: &str) -> Result<()>;

    async fn status(&self, namespace: &str) -> Result<Vec<ResourceStatus>>;

    fn backend_type(&self) -> &'static str;
}

pub struct ServiceSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<u16>,
    pub env: HashMap<String, String>,
    pub volumes: Vec<VolumeMount>,
}

pub struct ResourceHandle {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>,
}
```

**Design Rationale**:
- `async_trait` enables async methods in traits
- `Send + Sync` ensures backend can be shared across threads
- `provision()` returns handles for state tracking
- `deprovision()` is idempotent (safe to call multiple times)

#### 2.5.2 Docker Backend Implementation

```rust
// src/backend/docker.rs
use bollard::Docker;
use bollard::container::{CreateContainerOptions, Config, StartContainerOptions};

pub struct DockerBackend {
    client: Docker,
}

impl DockerBackend {
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_local_defaults()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Backend for DockerBackend {
    async fn provision(
        &self,
        namespace: &str,
        services: &HashMap<String, ServiceSpec>,
    ) -> Result<Vec<ResourceHandle>> {
        let mut handles = Vec::new();

        for (name, spec) in services {
            let container_name = format!("{}-{}", namespace, name);

            // Create container with labels
            let config = Config {
                image: Some(spec.image.clone()),
                env: Some(spec.env.iter().map(|(k, v)| format!("{}={}", k, v)).collect()),
                labels: Some(HashMap::from([
                    ("aether.managed".to_string(), "true".to_string()),
                    ("aether.workspace".to_string(), namespace.to_string()),
                    ("aether.service".to_string(), name.clone()),
                ])),
                ..Default::default()
            };

            let container = self.client.create_container(
                Some(CreateContainerOptions { name: container_name.clone(), ..Default::default() }),
                config,
            ).await?;

            // Start container
            self.client.start_container(&container.id, None::<StartContainerOptions<String>>).await?;

            // Get assigned ports
            let inspect = self.client.inspect_container(&container.id, None).await?;
            let port_mappings = extract_port_mappings(&inspect)?;

            handles.push(ResourceHandle {
                service_name: name.clone(),
                container_id: container.id,
                image: spec.image.clone(),
                port_mappings,
            });
        }

        Ok(handles)
    }

    async fn deprovision(&self, namespace: &str) -> Result<()> {
        // List containers with label filter
        let filters = HashMap::from([
            ("label", vec![format!("aether.workspace={}", namespace)])
        ]);

        let containers = self.client.list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        })).await?;

        for container in containers {
            if let Some(id) = container.id {
                // Force remove container
                self.client.remove_container(&id, Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                })).await?;
            }
        }

        Ok(())
    }

    fn backend_type(&self) -> &'static str {
        "docker"
    }
}
```

**Key Design Decisions**:
- Use `bollard` crate for type-safe Docker API
- Container labels enable cleanup and orphan detection
- Force remove on deprovision (don't wait for graceful shutdown)
- Namespace-based filtering for isolation

#### 2.5.3 SSH Docker Backend (Future)

```rust
// src/backend/ssh_docker.rs
pub struct SshDockerBackend {
    host: String,
    user: String,
    session: Mutex<ssh2::Session>,
}

// Implementation executes Docker CLI commands over SSH
// Similar interface to DockerBackend but uses SSH channel
```

## 3. Data Flow Diagrams

### 3.1 Workspace Add Flow

```
┌─────────┐
│  User   │
└────┬────┘
     │ ajj workspace add feature-x
     ▼
┌─────────────────┐
│ CLI Parser      │
└────┬────────────┘
     │ WorkspaceAction::Add { destination: "feature-x" }
     ▼
┌─────────────────┐
│ Config Loader   │ ──→ Find & parse aether.toml
└────┬────────────┘
     │ AetherConfig
     ▼
┌─────────────────┐
│ JJ Delegation   │ ──→ Execute: jj workspace add feature-x
└────┬────────────┘
     │ Success
     ▼
┌─────────────────────┐
│ Resource Provisioner│
│ ┌─────────────────┐ │
│ │ Port Allocator  │ │ ──→ Allocate 2 ports: [32891, 32892]
│ └─────────────────┘ │
│         │           │
│         ▼           │
│ ┌─────────────────┐ │
│ │ Backend         │ │ ──→ Docker API: Create & start containers
│ └─────────────────┘ │     with port mappings
│         │           │
│         ▼           │
│ ┌─────────────────┐ │
│ │ Context Injector│ │ ──→ Render template & write .env
│ └─────────────────┘ │
│         │           │
│         ▼           │
│ ┌─────────────────┐ │
│ │ State Manager   │ │ ──→ Register workspace in state.json
│ └─────────────────┘ │
└────┬────────────────┘
     │ WorkspaceInfo
     ▼
┌─────────────────┐
│ JSON Formatter  │ ──→ Output to stdout
└─────────────────┘
```

### 3.2 Workspace Forget Flow

```
User → CLI Parser → State Manager (load workspace)
                         │
                         ▼
                    Backend::deprovision() → Docker API (remove containers)
                         │
                         ▼
                    State Manager (unregister)
                         │
                         ▼
                    JJ Delegation (jj workspace forget)
                         │
                         ▼
                    JSON Formatter → Output
```

## 4. Error Handling Strategy

### 4.1 Error Type Hierarchy

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AetherError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Jujutsu command failed: {message} (exit code: {exit_code})")]
    Jj { message: String, exit_code: i32 },

    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),

    #[error("Port allocation failed: {0}")]
    PortAllocation(String),

    #[error("Context injection failed: {0}")]
    ContextInjection(String),

    #[error("State management error: {0}")]
    State(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AetherError>;
```

### 4.2 Error to JSON Conversion

```rust
// src/output/json.rs
impl From<AetherError> for ErrorInfo {
    fn from(err: AetherError) -> Self {
        match err {
            AetherError::Jj { message, exit_code } => ErrorInfo {
                code: "JJ_FAILED".to_string(),
                message,
                details: Some(json!({ "exit_code": exit_code })),
            },
            AetherError::Config(msg) => ErrorInfo {
                code: "CONFIG_ERROR".to_string(),
                message: msg,
                details: None,
            },
            // ... other variants
        }
    }
}
```

## 5. Testing Strategy

### 5.1 Unit Tests

**Port Allocator**:
```rust
#[tokio::test]
async fn test_concurrent_allocation() {
    let allocator = Arc::new(PortAllocator::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let alloc = allocator.clone();
        handles.push(tokio::spawn(async move {
            alloc.allocate(5).unwrap()
        }));
    }

    let results: Vec<Vec<u16>> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Verify no duplicate ports
    let all_ports: HashSet<u16> = results.into_iter().flatten().collect();
    assert_eq!(all_ports.len(), 50);
}
```

**Context Injector**:
```rust
#[test]
fn test_template_rendering() {
    let injector = ContextInjector::new();
    let mut resources = HashMap::new();
    resources.insert("postgres".to_string(), ResourceHandle {
        service_name: "postgres".to_string(),
        container_id: "abc123".to_string(),
        image: "postgres:15".to_string(),
        port_mappings: HashMap::from([(5432, 32891)]),
    });

    let template = "DB_PORT={{ services.postgres.ports.5432 }}";
    let result = injector.render(template, &resources).unwrap();
    assert_eq!(result, "DB_PORT=32891");
}
```

### 5.2 Integration Tests

**Full Workspace Lifecycle**:
```rust
#[tokio::test]
async fn test_workspace_lifecycle() {
    // Setup: Create test repo with aether.toml
    let temp_dir = TempDir::new().unwrap();
    setup_test_repo(&temp_dir);

    // Test: Create workspace
    let output = Command::new("ajj")
        .args(&["workspace", "add", "test-ws"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Verify: Container running
    let containers = list_docker_containers_with_label("aether.workspace=test-ws").await;
    assert_eq!(containers.len(), 1);

    // Test: Forget workspace
    let output = Command::new("ajj")
        .args(&["workspace", "forget", "test-ws"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Verify: Container removed
    let containers = list_docker_containers_with_label("aether.workspace=test-ws").await;
    assert_eq!(containers.len(), 0);
}
```

### 5.3 Test Fixtures

```rust
// tests/fixtures/mod.rs
pub fn create_test_config() -> String {
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

## 6. Performance Considerations

### 6.1 Optimization Targets

- **CLI Startup**: < 100ms (lazy load heavy dependencies)
- **Config Parsing**: < 10ms (cache parsed config)
- **Port Allocation**: < 50ms for 10 ports
- **Container Spawn**: < 2s per container (Docker-dependent)
- **State File I/O**: < 20ms with lock acquisition

### 6.2 Scalability Limits

- **Max Workspaces**: ~1000 (limited by state file size and lock contention)
- **Max Services per Workspace**: ~50 (practical limit)
- **Max Concurrent Operations**: ~10 (file lock contention)

## 7. Security Considerations

### 7.1 Credential Redaction

```rust
fn redact_env_vars(env: &HashMap<String, String>) -> HashMap<String, String> {
    const SENSITIVE_PATTERNS: &[&str] = &["PASSWORD", "SECRET", "TOKEN", "KEY"];

    env.iter()
        .map(|(k, v)| {
            if SENSITIVE_PATTERNS.iter().any(|p| k.to_uppercase().contains(p)) {
                (k.clone(), "***REDACTED***".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}
```

### 7.2 Command Injection Prevention

```rust
// CORRECT: Use Command::arg() (no shell interpolation)
Command::new("docker")
    .arg("run")
    .arg(&user_input)
    .output()?;

// WRONG: Shell interpolation (vulnerable)
// Command::new("sh").arg("-c").arg(format!("docker run {}", user_input))
```

## 8. Dependencies

### 8.1 Core Dependencies

```toml
[dependencies]
clap = { version = "4.4", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
anyhow = "1.0"
thiserror = "1.0"
async-trait = "0.1"

# Docker backend
bollard = "0.16"

# SSH backend (future)
ssh2 = "0.9"

# State management
fs2 = "0.4"  # File locking

# Template rendering
handlebars = "5.0"
```

### 8.2 External Tools

- **jujutsu**: Version >= 0.12.0
- **docker**: Docker Engine >= 20.10 (for Docker backend)

---

**Design Status**: ✅ READY FOR IMPLEMENTATION
