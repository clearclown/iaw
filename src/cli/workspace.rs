use crate::backend::{Backend, DockerBackend, ServiceSpec};
use crate::config::{find_config, load_config};
use crate::error::Result;
use crate::jj::JjCommand;
use crate::output::json::{AjjOutput, ResourceDetail, WorkspaceInfo};
use crate::provisioner::{
    context_injector, ContextInjector, PortAllocator, ResourceInfo, StateManager, WorkspaceState,
};
use crate::repo::find_repo_root;
use std::collections::HashMap;
use std::path::Path;

/// Parse memory string (e.g., "512m", "1g") to bytes
fn parse_memory_to_bytes(mem: &str) -> Result<i64> {
    let mem = mem.trim().to_lowercase();
    let (num_str, unit) = if mem.ends_with('b') {
        (&mem[..mem.len() - 1], &mem[mem.len() - 2..])
    } else {
        (&mem[..mem.len() - 1], &mem[mem.len() - 1..])
    };

    let num: f64 = num_str
        .parse()
        .map_err(|_| crate::error::AetherError::Config(format!("Invalid memory value: {}", mem)))?;

    let multiplier = match unit {
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(crate::error::AetherError::Config(format!(
                "Invalid memory unit: {}",
                unit
            )))
        }
    };

    Ok((num * multiplier) as i64)
}

pub async fn handle_workspace_add(
    destination: &str,
    revision: Option<&str>,
    config_path: Option<&str>,
    json: bool,
) -> Result<()> {
    // 1. Find and load config
    let config_file = if let Some(path) = config_path {
        std::path::PathBuf::from(path)
    } else {
        find_config(Path::new("."))?
    };
    let config = load_config(&config_file)?;

    // 2. Execute jj workspace add
    let jj_cmd = JjCommand::workspace_add(destination, revision);
    jj_cmd.execute()?;

    // 3. Generate namespace
    let workspace_name = Path::new(destination)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| crate::error::AetherError::Config("Invalid destination".into()))?;
    let namespace = format!("aether-{}", workspace_name);

    // 4. Count total ports needed
    let total_ports: usize = config.services.values().map(|s| s.ports.len()).sum();

    // 5. Allocate ports
    let allocator = PortAllocator::new();
    let allocated_ports = allocator.allocate(total_ports)?;

    // 6. Build service specs with port mappings
    let mut services = HashMap::new();
    let mut port_idx = 0;

    for (name, svc_config) in &config.services {
        let mut port_mappings = HashMap::new();

        for port_str in &svc_config.ports {
            let internal_port: u16 = port_str.parse().map_err(|_| {
                crate::error::AetherError::Config(format!("Invalid port: {}", port_str))
            })?;

            port_mappings.insert(internal_port, allocated_ports[port_idx]);
            port_idx += 1;
        }

        services.insert(
            name.clone(),
            ServiceSpec {
                name: name.clone(),
                image: svc_config.image.clone(),
                ports: svc_config
                    .ports
                    .iter()
                    .filter_map(|p| p.parse().ok())
                    .collect(),
                env: svc_config.env.clone(),
                volumes: svc_config.volumes.clone(),
                command: svc_config.command.clone(),
                port_mappings,
                depends_on: svc_config.depends_on.clone(),
                cpu_limit: svc_config.resources.as_ref().and_then(|r| r.cpu_limit),
                cpu_reservation: svc_config
                    .resources
                    .as_ref()
                    .and_then(|r| r.cpu_reservation),
                memory_limit: svc_config
                    .resources
                    .as_ref()
                    .and_then(|r| r.memory_limit.as_ref())
                    .and_then(|m| parse_memory_to_bytes(m).ok()),
                memory_reservation: svc_config
                    .resources
                    .as_ref()
                    .and_then(|r| r.memory_reservation.as_ref())
                    .and_then(|m| parse_memory_to_bytes(m).ok()),
            },
        );
    }

    // 7. Provision via backend
    let backend = DockerBackend::new()?;
    let handles = backend.provision(&namespace, &services).await?;

    // 8. Inject context if configured
    if let Some(injection_config) = &config.injection {
        let injector = ContextInjector::new();
        let mut resources = HashMap::new();

        for handle in &handles {
            resources.insert(
                handle.service_name.clone(),
                context_injector::ResourceHandle {
                    service_name: handle.service_name.clone(),
                    container_id: handle.container_id.clone(),
                    image: handle.image.clone(),
                    port_mappings: handle.port_mappings.clone(),
                },
            );
        }

        let rendered = injector.render(&injection_config.template, &resources)?;
        let dest_path = Path::new(destination).join(&injection_config.file);
        std::fs::write(dest_path, rendered)?;
    }

    // 9. Register workspace
    let repo_root = find_repo_root(Path::new("."))?;
    let state_manager = StateManager::new(&repo_root);

    let workspace_state = WorkspaceState {
        name: workspace_name.to_string(),
        path: std::fs::canonicalize(destination)?
            .to_string_lossy()
            .to_string(),
        namespace: namespace.clone(),
        backend_type: "docker".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        resources: handles
            .iter()
            .map(|h| ResourceInfo {
                service_name: h.service_name.clone(),
                container_id: h.container_id.clone(),
                image: h.image.clone(),
                port_mappings: h.port_mappings.clone(),
            })
            .collect(),
    };

    state_manager.register_workspace(workspace_state)?;

    // 10. Output
    if json {
        let output = AjjOutput {
            status: "ready".to_string(),
            operation: "workspace_add".to_string(),
            workspace: Some(WorkspaceInfo {
                name: workspace_name.to_string(),
                root: std::fs::canonicalize(destination)?
                    .to_string_lossy()
                    .to_string(),
                backend: "docker".to_string(),
                namespace: namespace.clone(),
                resources: handles
                    .iter()
                    .map(|h| ResourceDetail {
                        service_name: h.service_name.clone(),
                        container_id: h.container_id.clone(),
                        image: h.image.clone(),
                        port_mappings: h.port_mappings.clone(),
                    })
                    .collect(),
            }),
            errors: vec![],
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "✓ Workspace '{}' created with {} containers",
            workspace_name,
            handles.len()
        );
    }

    Ok(())
}

pub async fn handle_workspace_forget(workspace: &str, json: bool) -> Result<()> {
    // 1. Find repo root and load state
    let repo_root = find_repo_root(Path::new("."))?;
    let state_manager = StateManager::new(&repo_root);

    // 2. Get workspace state
    let workspace_state = state_manager.get_workspace(workspace)?;

    // 3. Deprovision if found
    let removed_count = if let Some(state) = workspace_state {
        let backend = DockerBackend::new()?;
        backend.deprovision(&state.namespace).await?;
        state_manager.unregister_workspace(workspace)?;
        state.resources.len()
    } else {
        if !json {
            println!("⚠ Workspace not found in state (continuing with jj operation)");
        }
        0
    };

    // 4. Execute jj workspace forget
    let jj_cmd = JjCommand::workspace_forget(workspace);
    jj_cmd.execute()?;

    if json {
        let output = AjjOutput {
            status: "removed".to_string(),
            operation: "workspace_forget".to_string(),
            workspace: None,
            errors: vec![],
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✓ Cleaned up {} containers", removed_count);
        println!("✓ Workspace '{}' forgotten", workspace);
    }

    Ok(())
}
