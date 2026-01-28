# Phase 3: Requirements Alignment & Validation

## Purpose
This document verifies that the extracted requirements (Phase 1) and proposed structure (Phase 2) are consistent, complete, and implementable without conflicts.

## Alignment Matrix

| Requirement ID | Design Component | Status | Notes |
|----------------|------------------|--------|-------|
| FR-WS-001 | cli/workspace.rs, provisioner/manager.rs | Aligned | Workspace creation flow fully specified |
| FR-WS-002 | cli/workspace.rs, backend/trait.rs deprovision | Aligned | Cleanup mechanism defined |
| FR-PORT-001 | provisioner/port_allocator.rs | Aligned | Dynamic port allocation implemented |
| FR-PORT-002 | provisioner/state.rs | Aligned | Port mappings persisted in WorkspaceState |
| FR-CTX-001 | provisioner/context_injector.rs | Aligned | Handlebars template engine specified |
| FR-CTX-002 | output/json.rs | Aligned | JSON schema defined |
| FR-EXEC-001 | cli/run.rs | Aligned | Environment loading specified |
| FR-BE-001 | backend/trait.rs, backend modules | Aligned | Trait-based abstraction |
| FR-BE-002 | config/schema.rs BackendConfig | Aligned | Tagged enum for backend types |
| FR-CFG-001 | config/schema.rs AetherConfig | Aligned | TOML schema matches example |

## Consistency Checks

### 1. Configuration to Runtime Mapping
**Question**: Can aether.toml services be translated to backend ServiceSpec?

**Analysis**:
- Image: Direct mapping (ServiceConfig.image to ServiceSpec.image)
- Ports: List of strings parsed to vector of u16
- Env: Direct mapping (HashMap String to String)
- Volumes: String array parsed to vector of VolumeMount
- Command: Optional vector of String

**Verdict**: Complete mapping exists

### 2. State Persistence
**Question**: Is workspace state sufficient to reconstruct environment?

**Analysis**:
WorkspaceState contains:
- namespace: For backend filtering
- backend_type: To instantiate correct backend
- resources container_id: For container operations
- resources port_mappings: For env regeneration

**Missing**:
- Original service configuration (image, env vars)
- Impact: Cannot recreate containers from state alone
- Resolution: Store original config hash or reference to aether.toml

**Verdict**: Requires enhancement - add config_hash field to WorkspaceState

### 3. Backend Abstraction Completeness
**Question**: Does Backend trait cover all required operations?

**Analysis**:
- provision(): Create resources - covered
- deprovision(): Destroy resources - covered
- status(): Query resource state - covered
- Missing: logs() - No log retrieval method
- Missing: container execution - Cannot run commands inside containers

**Verdict**: Trait incomplete for debugging use cases

**Recommendation**: Add optional methods for logs and container execution

### 4. Port Allocation Race Conditions
**Question**: Is PortAllocator safe for concurrent use?

**Analysis**:
- Current design uses HashSet without synchronization
- Issue: Two concurrent workspace add calls may allocate same port
- Resolution: Wrap in Mutex or use atomic operations

**Verdict**: Requires thread-safety enhancement with Mutex wrapper

### 5. Error Propagation
**Question**: Can all errors be represented in JSON output?

**Analysis**:
- AetherError variants have descriptive messages
- ErrorInfo struct supports code plus message plus details
- Error conversion from BackendError to AetherError to ErrorInfo

**Verdict**: Consistent error handling

### 6. JJ Command Delegation
**Question**: How to handle jj output for AI consumption?

**Analysis**:
- jj CLI already supports color=never for plain output
- No mention of capturing jj stderr separately
- Issue: jj errors may mix with ajj messages

**Recommendation**: Capture stdout and stderr separately, parse jj errors into ErrorInfo

### 7. Namespace Collision Avoidance
**Question**: Is namespace generation unique?

**Design**: aether-<repo_hash>-<workspace_name>

**Analysis**:
- repo_hash: Unique per repository
- workspace_name: User-provided, could be duplicated across repos
- Collision scenario: Two repos with workspace feature-x
- Resolution: Namespace includes repo hash, so no collision

**Verdict**: Collision-resistant

### 8. Config File Discovery
**Question**: How does ajj find aether.toml in nested directories?

**Current Design**:
Default value "aether.toml" in config argument

**Issue**:
- Does not search parent directories
- Impact: Running ajj from subdirectory fails

**Resolution**: Implement config file search algorithm:
1. Check current directory
2. Walk up to repo root (detected via .jj directory)
3. Error if not found

**Verdict**: Requires enhancement

## Gap Analysis

### Identified Gaps

1. **State Storage Location**
   - Issue: No specification of where WorkspaceRegistry is stored
   - Options:
     - A: .aether/state.json in repo root
     - B: home config aether state.db (global)
   - Recommendation: Option A (repo-local) for multi-repo support
   - Priority: P0 (blocking)

2. **State Locking**
   - Issue: Concurrent modification of state file
   - Resolution: File-based locking (fs2 crate)
   - Priority: P0 (blocking)

3. **Container Labeling**
   - Issue: How to identify Aether-managed containers?
   - Resolution: Add Docker labels:
     - aether.workspace=<name>
     - aether.namespace=<namespace>
     - aether.managed=true
   - Priority: P1 (required for cleanup)

4. **Orphan Detection**
   - Issue: No mechanism to detect containers from crashed ajj processes
   - Resolution: ajj cleanup command:
     1. List all containers with aether.managed=true
     2. Cross-reference with WorkspaceRegistry
     3. Offer to remove orphans
   - Priority: P1 (operational reliability)

5. **SSH Key Management**
   - Issue: BackendConfig Ssh has key_path but no password auth
   - Resolution: Add auth enum with Key, Password, and Agent variants
   - Priority: P2 (UX improvement)

6. **Volume Path Resolution**
   - Issue: Relative volume paths in aether.toml need resolution
   - Example: volumes = ["./data:/data"]
   - Resolution: Resolve relative to aether.toml location
   - Priority: P1 (functional correctness)

7. **Health Checks**
   - Issue: No way to wait for service readiness
   - Example: Postgres takes 5s to start, but env is written immediately
   - Resolution: Add optional health_check config with test, interval, timeout
   - Priority: P2 (reliability)

8. **Template Functions**
   - Issue: Template only supports port mapping, no other helpers
   - Example: Cannot generate random passwords
   - Resolution: Add Handlebars helpers: random_string, base64
   - Priority: P3 (feature enhancement)

## Conflict Resolution

### Conflict 1: Output Format Control
**Issue**: Global output json flag vs. command-specific json flags

**Resolution**: Remove command-specific flags, use global flag only

**Impact**: Simplifies CLI, consistent behavior

### Conflict 2: Workspace Identification
**Issue**: Commands accept both name and path

**Resolution**: Implement resolver:
1. If argument contains slash, treat as path, resolve to absolute
2. Otherwise, treat as workspace name, look up in registry
3. If ambiguous, error with suggestions

### Conflict 3: Environment Variable Precedence
**Issue**: env file vs. system env vars

**Scenario**:
- env file has DATABASE_URL=localhost:32891
- System has DATABASE_URL=prod.db.com

**Resolution**: System env takes precedence (standard behavior), document clearly

## Validation Criteria

### Completeness
- All functional requirements mapped to design components
- Data flow from CLI to Backend to Output specified
- Error paths defined
- State management strategy clear

### Consistency
- Configuration schema matches example aether.toml
- JSON output schema matches AI requirements
- Backend trait covers core operations
- Action Required: Add config_hash to WorkspaceState
- Action Required: Make PortAllocator thread-safe
- Action Required: Implement config file discovery

### Feasibility
- Rust ecosystem has required libraries:
  - clap (CLI)
  - tokio (async)
  - bollard (Docker API)
  - handlebars (templates)
  - serde and toml (config)
- External dependencies (jj, docker) clearly specified
- No architectural impossibilities identified

### Testability
- Unit test boundaries clear (port allocator, context injector)
- Integration test scenarios defined
- Action Required: Add test fixtures for Docker mocking
- Action Required: Define test harness for backend trait

## Risk Mitigation Updates

### Risk 1: Port Allocation Races (Original)
**Updated Mitigation**: Mutex-protected PortAllocator plus file-based state locking

### Risk 2: Orphaned Containers (Original)
**Updated Mitigation**: Docker labels plus ajj cleanup command

### NEW Risk 3: State File Corruption
**Issue**: Partially written JSON state on crash
**Mitigation**: Atomic write (write to .tmp, then rename)

### NEW Risk 4: Backend API Changes
**Issue**: Docker API or SSH protocol changes break backends
**Mitigation**:
- Pin dependency versions
- Abstract API calls behind internal wrappers
- Version backend implementations

## Alignment Summary

### Green Light (Proceed as Designed)
- CLI structure and command parsing
- JSON output schema
- Context injection with Handlebars
- Backend trait abstraction (with additions)
- Configuration schema

### Yellow Light (Needs Minor Adjustments)
- WorkspaceState: Add config_hash field
- PortAllocator: Add thread-safety (Mutex)
- Backend Trait: Add optional logs and container execution methods
- Config Discovery: Implement parent directory search

### Red Light (Requires Rethinking)
- None identified

## Implementation Readiness

**Overall Assessment**: Ready to proceed with adjustments

**Prerequisites Before Coding**:
1. Create state file schema with locking mechanism
2. Add Docker label specification to backend implementations
3. Define test fixture structure
4. Document config discovery algorithm

**Estimated Adjustments**: 4-6 hours of additional design work

## Next Phase Preparation

Phase 4 (Verification) should focus on:
1. Finalizing state management implementation details
2. Creating detailed test plan with edge cases
3. Writing comprehensive API documentation for backend trait
4. Producing implementation task breakdown

All core architectural decisions are validated and ready for specification finalization.
