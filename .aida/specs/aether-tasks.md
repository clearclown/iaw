# Aether (ajj) - Implementation Tasks

## Document Information
- **Project**: Aether - Infrastructure as Workspace
- **Version**: 1.0.0-MVP
- **Date**: 2026-01-28

## Task Organization

Tasks are organized by phase and module. Each task includes:
- **ID**: Unique identifier
- **Priority**: P0 (Critical), P1 (High), P2 (Medium), P3 (Low)
- **Estimated Effort**: S (Small, <4h), M (Medium, 4-8h), L (Large, 8-16h), XL (Extra Large, >16h)
- **Dependencies**: Tasks that must be completed first
- **Acceptance Criteria**: How to verify completion

---

## Phase 1: Project Scaffolding

### TASK-001: Initialize Rust Project
**Priority**: P0
**Effort**: S
**Dependencies**: None

**Description**: Create Cargo project structure with proper organization

**Steps**:
1. Run `cargo new --bin aether`
2. Configure `Cargo.toml` with binary name `ajj`
3. Set up workspace module structure
4. Add initial dependencies

**Acceptance Criteria**:
- `cargo build` succeeds
- Binary is named `ajj`
- Project structure matches design document

**Cargo.toml**:
```toml
[package]
name = "aether"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "ajj"
path = "src/main.rs"

[dependencies]
clap = { version = "4.4", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
anyhow = "1.0"
thiserror = "1.0"
async-trait = "0.1"
bollard = "0.16"
fs2 = "0.4"
handlebars = "5.0"
```

---

### TASK-002: Create Module Structure
**Priority**: P0
**Effort**: S
**Dependencies**: TASK-001

**Description**: Create all module directories and mod.rs files

**Steps**:
1. Create `src/cli/`, `src/config/`, `src/jj/`, `src/provisioner/`, `src/backend/`, `src/output/`
2. Create `mod.rs` in each directory
3. Export modules in `src/lib.rs`
4. Create `src/error.rs` with error types

**File Tree**:
```
src/
├── main.rs
├── lib.rs
├── error.rs
├── cli/
│   ├── mod.rs
│   ├── commands.rs
│   ├── workspace.rs
│   ├── run.rs
│   └── status.rs
├── config/
│   ├── mod.rs
│   ├── schema.rs
│   ├── loader.rs
│   └── validation.rs
├── jj/
│   ├── mod.rs
│   ├── delegation.rs
│   └── parser.rs
├── provisioner/
│   ├── mod.rs
│   ├── manager.rs
│   ├── port_allocator.rs
│   ├── context_injector.rs
│   └── state.rs
├── backend/
│   ├── mod.rs
│   ├── trait.rs
│   ├── docker.rs
│   └── ssh_docker.rs (future)
└── output/
    ├── mod.rs
    ├── json.rs
    └── human.rs
```

**Acceptance Criteria**:
- `cargo build` succeeds with all modules
- No warnings about unused modules

---

### TASK-003: Implement Error Types
**Priority**: P0
**Effort**: S
**Dependencies**: TASK-002

**Description**: Define comprehensive error types using `thiserror`

**Implementation**: See design doc `src/error.rs`

**Acceptance Criteria**:
- All error variants defined
- Implements `std::error::Error`
- Has `Display` formatting
- Unit test for error conversion

---

## Phase 2: Core Infrastructure

### TASK-004: Configuration Schema
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-003

**Description**: Implement TOML configuration schema with serde

**Files**: `src/config/schema.rs`

**Steps**:
1. Define `AetherConfig`, `BackendConfig`, `ServiceConfig`, `InjectionConfig` structs
2. Add serde derive macros
3. Implement validation methods
4. Add builder pattern for tests

**Acceptance Criteria**:
- Example `aether.toml` parses correctly
- Invalid config produces clear error messages
- Unit tests for all config variants

---

### TASK-005: Config File Discovery
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-004

**Description**: Implement config file search algorithm

**Files**: `src/config/loader.rs`

**Steps**:
1. Implement `find_config()` function (walk up to `.jj` directory)
2. Implement `load_config()` function (parse TOML)
3. Handle `--config` flag override
4. Add caching for repeated access

**Acceptance Criteria**:
- Finds config in parent directories
- Stops at repo root
- Respects CLI flag override
- Returns helpful error if not found
- Unit tests with temp directories

---

### TASK-006: State Management
**Priority**: P0
**Effort**: L
**Dependencies**: TASK-003

**Description**: Implement workspace state registry with file locking

**Files**: `src/provisioner/state.rs`

**Steps**:
1. Define `WorkspaceRegistry` and `WorkspaceState` structs
2. Implement `StateManager` with file locking (`fs2` crate)
3. Implement atomic write (write to .tmp, then rename)
4. Add `register_workspace()`, `unregister_workspace()`, `get_workspace()`, `list_workspaces()`
5. Handle corrupted state file recovery

**Acceptance Criteria**:
- Concurrent writes don't corrupt state
- Lock is released on panic
- State file is human-readable JSON
- Unit tests for concurrent access
- Integration test for atomic writes

---

### TASK-007: Port Allocator
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-003

**Description**: Implement thread-safe dynamic port allocation

**Files**: `src/provisioner/port_allocator.rs`

**Steps**:
1. Implement `PortAllocator` with `Mutex<HashSet<u16>>`
2. Use `TcpListener::bind("127.0.0.1:0")` for OS port assignment
3. Add `allocate(count)` and `release(ports)` methods
4. Handle allocation failures gracefully

**Acceptance Criteria**:
- Allocates N unique ports
- Thread-safe (verified by concurrent test)
- No port collisions in 1000 allocations
- Proper cleanup on release

**Test**:
```rust
#[tokio::test]
async fn test_no_collision() {
    let allocator = Arc::new(PortAllocator::new());
    let mut handles = vec![];

    for _ in 0..100 {
        let a = allocator.clone();
        handles.push(tokio::spawn(async move {
            a.allocate(5).unwrap()
        }));
    }

    let results: Vec<Vec<u16>> = futures::future::join_all(handles)
        .await.into_iter().map(|r| r.unwrap()).collect();

    let all_ports: HashSet<u16> = results.into_iter().flatten().collect();
    assert_eq!(all_ports.len(), 500); // No duplicates
}
```

---

### TASK-008: Context Injector
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-003

**Description**: Implement template rendering with Handlebars

**Files**: `src/provisioner/context_injector.rs`

**Steps**:
1. Initialize `Handlebars` instance
2. Implement `render()` method
3. Build context from `ResourceHandle` map
4. Handle template errors with helpful messages
5. Write rendered output to `.env` file

**Acceptance Criteria**:
- Correctly substitutes port mappings
- Handles missing services gracefully
- Produces actionable error for invalid templates
- Unit tests for various templates

**Test**:
```rust
#[test]
fn test_multi_service_template() {
    let injector = ContextInjector::new();
    let resources = HashMap::from([
        ("postgres".to_string(), ResourceHandle {
            service_name: "postgres".to_string(),
            container_id: "abc".to_string(),
            image: "postgres:15".to_string(),
            port_mappings: HashMap::from([(5432, 32891)]),
        }),
        ("redis".to_string(), ResourceHandle {
            service_name: "redis".to_string(),
            container_id: "def".to_string(),
            image: "redis:7".to_string(),
            port_mappings: HashMap::from([(6379, 32892)]),
        }),
    ]);

    let template = r#"
DATABASE_URL=postgres://localhost:{{ services.postgres.ports.5432 }}
REDIS_URL=redis://localhost:{{ services.redis.ports.6379 }}
"#;
    let result = injector.render(template, &resources).unwrap();
    assert!(result.contains("32891"));
    assert!(result.contains("32892"));
}
```

---

## Phase 3: JJ Integration

### TASK-009: JJ Command Delegation
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-003

**Description**: Implement subprocess execution for jj commands

**Files**: `src/jj/delegation.rs`

**Steps**:
1. Implement `JjCommand` struct with builder pattern
2. Add methods for common operations (`workspace_add`, `workspace_forget`, `status`)
3. Use `std::process::Command` with separate stdout/stderr capture
4. Parse jj errors from stderr
5. Handle `jj` not found error

**Acceptance Criteria**:
- Successfully executes `jj workspace add`
- Captures stdout and stderr separately
- Returns clear error if jj not in PATH
- Unit tests with mock jj binary
- Integration test with real jj

---

### TASK-010: JJ Output Parsing
**Priority**: P1
**Effort**: S
**Dependencies**: TASK-009

**Description**: Parse jj command output for relevant information

**Files**: `src/jj/parser.rs`

**Steps**:
1. Implement parser for `jj status` output
2. Extract workspace path from output
3. Handle different jj versions gracefully

**Acceptance Criteria**:
- Extracts workspace root path
- Handles empty repositories
- Unit tests with sample jj output

---

## Phase 4: Backend Implementation

### TASK-011: Backend Trait Definition
**Priority**: P0
**Effort**: S
**Dependencies**: TASK-003

**Description**: Define async trait for backend abstraction

**Files**: `src/backend/trait.rs`

**Steps**:
1. Define `Backend` trait with `async_trait`
2. Define `ServiceSpec`, `ResourceHandle`, `ResourceStatus` types
3. Define `BackendError` enum

**Implementation**: See design doc

**Acceptance Criteria**:
- Trait compiles with `async_trait`
- All types implement required traits (Clone, Debug, etc.)

---

### TASK-012: Docker Backend Implementation
**Priority**: P0
**Effort**: XL
**Dependencies**: TASK-011

**Description**: Implement Docker backend using bollard

**Files**: `src/backend/docker.rs`

**Steps**:
1. Implement `DockerBackend` struct with `bollard::Docker` client
2. Implement `provision()`:
   - Create containers with labels
   - Configure port mappings
   - Start containers
   - Extract assigned ports
3. Implement `deprovision()`:
   - List containers by label
   - Force remove all matching containers
4. Implement `status()`:
   - Query container state
   - Return structured status

**Container Labels**:
- `aether.managed=true`
- `aether.workspace=<workspace_name>`
- `aether.namespace=<namespace>`
- `aether.service=<service_name>`

**Acceptance Criteria**:
- Provisions containers with correct configuration
- Correctly maps ports
- Cleans up all containers on deprovision
- Integration test with real Docker daemon
- Handles Docker daemon unavailable error

**Test**:
```rust
#[tokio::test]
async fn test_docker_provision() {
    let backend = DockerBackend::new().unwrap();
    let services = HashMap::from([
        ("test".to_string(), ServiceSpec {
            name: "test".to_string(),
            image: "alpine:latest".to_string(),
            ports: vec![],
            env: HashMap::new(),
            volumes: vec![],
            command: Some(vec!["sleep".to_string(), "60".to_string()]),
        }),
    ]);

    let handles = backend.provision("test-namespace", &services).await.unwrap();
    assert_eq!(handles.len(), 1);

    // Cleanup
    backend.deprovision("test-namespace").await.unwrap();

    // Verify removed
    let status = backend.status("test-namespace").await.unwrap();
    assert_eq!(status.len(), 0);
}
```

---

### TASK-013: SSH Docker Backend (Phase 2)
**Priority**: P2
**Effort**: XL
**Dependencies**: TASK-012

**Description**: Implement remote Docker via SSH

**Files**: `src/backend/ssh_docker.rs`

**Steps**:
1. Use `ssh2` crate for SSH connections
2. Execute Docker commands over SSH channel
3. Parse command output
4. Handle SSH connection failures

**Acceptance Criteria**:
- Connects to remote Docker host
- Executes Docker commands remotely
- Handles authentication (key, password)
- Integration test with mock SSH server

---

## Phase 5: CLI Implementation

### TASK-014: CLI Argument Parsing
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-002

**Description**: Implement clap-based CLI structure

**Files**: `src/cli/commands.rs`

**Steps**:
1. Define `Cli` struct with clap derives
2. Define all subcommands
3. Add global flags (`--output`, `--config`)
4. Implement external subcommand for jj passthrough

**Implementation**: See design doc

**Acceptance Criteria**:
- `ajj --help` shows correct usage
- All subcommands parse correctly
- Unknown commands pass through to jj
- Unit tests for argument parsing

---

### TASK-015: Workspace Add Command
**Priority**: P0
**Effort**: L
**Dependencies**: TASK-005, TASK-006, TASK-007, TASK-008, TASK-009, TASK-012

**Description**: Implement `ajj workspace add` command

**Files**: `src/cli/workspace.rs`

**Steps**:
1. Load config
2. Validate destination path
3. Execute jj workspace add
4. Generate namespace
5. Allocate ports
6. Provision containers via backend
7. Inject context (.env file)
8. Register workspace in state
9. Output result

**Error Handling**: Rollback on failure (cleanup containers, release ports)

**Acceptance Criteria**:
- Creates workspace with containers
- Writes correct .env file
- Updates state registry
- Outputs JSON if requested
- Integration test with real Docker

---

### TASK-016: Workspace Forget Command
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-006, TASK-009, TASK-012

**Description**: Implement `ajj workspace forget` command

**Files**: `src/cli/workspace.rs`

**Steps**:
1. Resolve workspace name/path
2. Load workspace state
3. Deprovision containers
4. Unregister from state
5. Execute jj workspace forget
6. Output result

**Acceptance Criteria**:
- Removes all containers
- Updates state registry
- Handles non-existent workspace gracefully
- Integration test verifies cleanup

---

### TASK-017: Run Command
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-006, TASK-009

**Description**: Implement `ajj run -- <command>` command

**Files**: `src/cli/run.rs`

**Steps**:
1. Detect current workspace
2. Load .env file
3. Parse key=value pairs
4. Merge with system environment
5. Spawn subprocess with merged env
6. Stream stdout/stderr
7. Return subprocess exit code

**Acceptance Criteria**:
- Loads .env correctly
- System env takes precedence
- Preserves signals (SIGINT)
- Interactive commands work
- Exit code matches subprocess

---

### TASK-018: Status Command
**Priority**: P1
**Effort**: M
**Dependencies**: TASK-006, TASK-009, TASK-012

**Description**: Implement `ajj status` command

**Files**: `src/cli/status.rs`

**Steps**:
1. Run `jj status` and capture output
2. Load current workspace state
3. Query backend for resource status
4. Format output (human or JSON)

**Acceptance Criteria**:
- Shows both jj and infrastructure status
- JSON output is valid
- Handles non-Aether workspace gracefully

---

### TASK-019: List Command
**Priority**: P1
**Effort**: S
**Dependencies**: TASK-006

**Description**: Implement `ajj list` command

**Files**: `src/cli/commands.rs`

**Steps**:
1. Load state registry
2. Format as table or JSON
3. Show workspace name, path, backend, resource count

**Acceptance Criteria**:
- Lists all registered workspaces
- JSON and human-readable formats
- Handles empty list

---

### TASK-020: Cleanup Command
**Priority**: P1
**Effort**: M
**Dependencies**: TASK-006, TASK-012

**Description**: Implement `ajj cleanup` command

**Files**: `src/cli/commands.rs`

**Steps**:
1. Query backend for containers with `aether.managed=true` label
2. Load state registry
3. Identify orphans (containers not in state)
4. Display orphans
5. If `--force`, remove orphans and update state

**Acceptance Criteria**:
- Detects orphaned containers
- Dry-run shows what would be removed
- Force flag actually removes containers
- Updates state registry

---

## Phase 6: Output Formatting

### TASK-021: JSON Output Formatter
**Priority**: P0
**Effort**: M
**Dependencies**: TASK-003

**Description**: Implement structured JSON output

**Files**: `src/output/json.rs`

**Steps**:
1. Define `AjjOutput`, `WorkspaceInfo`, `ResourceInfo`, `ErrorInfo` structs
2. Implement `From<AetherError>` for `ErrorInfo`
3. Implement output formatting functions
4. Add pretty-print option

**Acceptance Criteria**:
- Outputs valid JSON
- Includes all required fields
- Error information is actionable
- Unit tests for schema

---

### TASK-022: Human-Readable Output
**Priority**: P1
**Effort**: S
**Dependencies**: None

**Description**: Implement user-friendly console output

**Files**: `src/output/human.rs`

**Steps**:
1. Format workspace creation with emoji/colors
2. Format status as table
3. Format errors with suggestions

**Acceptance Criteria**:
- Output is readable and helpful
- Colors work on supported terminals
- Degrades gracefully on non-TTY

---

## Phase 7: Testing

### TASK-023: Unit Test Suite
**Priority**: P0
**Effort**: L
**Dependencies**: All implementation tasks

**Description**: Comprehensive unit tests for all modules

**Coverage Target**: > 80%

**Key Tests**:
- Port allocator concurrency
- Context injection templates
- Config parsing
- Error conversion
- State management atomicity

**Acceptance Criteria**:
- `cargo test` passes
- Coverage > 80%
- No flaky tests

---

### TASK-024: Integration Test Suite
**Priority**: P0
**Effort**: XL
**Dependencies**: TASK-023

**Description**: End-to-end tests with real Docker

**Tests**:
1. Full workspace lifecycle (add → run → forget)
2. Multiple parallel workspaces
3. Orphan cleanup
4. Config validation
5. Error scenarios

**Setup**:
- Use `testcontainers` crate for isolated Docker
- Create temp repositories with jj
- Clean up after tests

**Acceptance Criteria**:
- All integration tests pass
- No leftover containers after tests
- Tests run in CI environment

---

### TASK-025: Documentation
**Priority**: P1
**Effort**: M
**Dependencies**: All tasks

**Description**: Write comprehensive documentation

**Deliverables**:
1. README.md with quickstart
2. CONTRIBUTING.md
3. API documentation (rustdoc)
4. Example aether.toml with comments
5. Troubleshooting guide

**Acceptance Criteria**:
- `cargo doc` generates full API docs
- README includes installation and usage
- Examples are tested

---

## Phase 8: Polishing

### TASK-026: Error Message Improvement
**Priority**: P2
**Effort**: M
**Dependencies**: TASK-023

**Description**: Review and improve all error messages

**Steps**:
1. Add suggestions to common errors
2. Include links to documentation
3. Test with AI agents for clarity

**Example**:
```
Error: jj command not found

Aether requires Jujutsu (jj) to be installed.

Install with:
  cargo install jj-cli

Or see: https://github.com/martinvonz/jj#installation
```

**Acceptance Criteria**:
- All errors include actionable suggestions
- AI agents can self-recover from errors

---

### TASK-027: Performance Optimization
**Priority**: P2
**Effort**: M
**Dependencies**: TASK-024

**Description**: Profile and optimize critical paths

**Targets**:
- CLI startup time < 100ms
- Workspace add overhead < 1s
- State file I/O < 20ms

**Tools**: Use `cargo flamegraph` for profiling

**Acceptance Criteria**:
- Performance targets met
- No unnecessary allocations in hot paths

---

### TASK-028: CI/CD Setup
**Priority**: P1
**Effort**: M
**Dependencies**: TASK-024

**Description**: GitHub Actions workflow

**Jobs**:
1. Lint (clippy)
2. Format (rustfmt)
3. Test (unit + integration)
4. Build (all platforms)
5. Release (cargo publish)

**Acceptance Criteria**:
- All checks run on PR
- Releases automated
- Coverage reports generated

---

## Task Summary

| Phase | Tasks | Total Effort |
|-------|-------|--------------|
| 1. Scaffolding | 3 | 3 S |
| 2. Core Infrastructure | 5 | 1 M + 3 M + 1 L = ~20h |
| 3. JJ Integration | 2 | 1 M + 1 S = ~5h |
| 4. Backend | 3 | 1 S + 1 XL + 1 XL = ~35h |
| 5. CLI | 7 | 1 M + 2 L + 4 M = ~30h |
| 6. Output | 2 | 1 M + 1 S = ~5h |
| 7. Testing | 3 | 1 L + 1 XL + 1 M = ~32h |
| 8. Polishing | 3 | 3 M = ~15h |

**Total Estimated Effort**: ~145 hours (3-4 weeks for 1 developer)

---

## Implementation Order (Critical Path)

1. TASK-001 → TASK-002 → TASK-003 (Scaffolding)
2. TASK-004 → TASK-005 (Config)
3. TASK-006, TASK-007, TASK-008 (State, Ports, Injection) [parallel]
4. TASK-009 (JJ Delegation)
5. TASK-011 → TASK-012 (Backend)
6. TASK-014 (CLI Parsing)
7. TASK-015 → TASK-016 → TASK-017 (Core Commands)
8. TASK-021 (JSON Output)
9. TASK-023 → TASK-024 (Testing)
10. TASK-028 (CI/CD)

**MVP Complete After**: Tasks 1-17, 21, 23, 24

---

**Task List Status**: ✅ READY FOR IMPLEMENTATION
