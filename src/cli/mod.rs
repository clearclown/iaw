pub mod commands;
pub mod completion;
pub mod logs;
pub mod run;
pub mod status;
pub mod workspace;

pub use commands::*;
pub use completion::*;
pub use logs::*;
pub use run::*;
pub use status::*;
pub use workspace::*;

use crate::error::Result;
use crate::output::json::CleanupOutput;
use crate::provisioner::StateManager;
use crate::repo::find_repo_root;
use std::path::Path;

pub async fn handle_list(json: bool) -> Result<()> {
    let repo_root = find_repo_root(Path::new("."))?;
    let state_manager = StateManager::new(&repo_root);

    let workspaces = state_manager.list_workspaces()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&workspaces)?);
    } else if workspaces.is_empty() {
        println!("No workspaces registered.");
    } else {
        println!("=== Workspaces ===");
        for ws in workspaces {
            println!(
                "  {} ({}, {} resources)",
                ws.name,
                ws.backend_type,
                ws.resources.len()
            );
            println!("    path: {}", ws.path);
            println!("    namespace: {}", ws.namespace);
        }
    }

    Ok(())
}

pub async fn handle_cleanup(force: bool, json: bool) -> Result<()> {
    use bollard::container::ListContainersOptions;
    use bollard::Docker;
    use std::collections::HashMap;

    let repo_root = find_repo_root(Path::new("."))?;
    let state_manager = StateManager::new(&repo_root);
    let registered_workspaces = state_manager.list_workspaces()?;

    let registered_namespaces: std::collections::HashSet<_> = registered_workspaces
        .iter()
        .map(|ws| ws.namespace.clone())
        .collect();

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| crate::error::AetherError::Backend(format!("Docker connect failed: {}", e)))?;

    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["aether.managed=true".to_string()]);

    let containers = docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        }))
        .await
        .map_err(|e| {
            crate::error::AetherError::Backend(format!("List containers failed: {}", e))
        })?;

    let mut orphans = Vec::new();
    for container in &containers {
        if let Some(labels) = &container.labels {
            if let Some(namespace) = labels.get("aether.namespace") {
                if !registered_namespaces.contains(namespace) {
                    orphans.push((
                        container.id.clone().unwrap_or_default(),
                        namespace.clone(),
                        labels.get("aether.service").cloned().unwrap_or_default(),
                    ));
                }
            }
        }
    }

    if orphans.is_empty() {
        if json {
            let output = CleanupOutput {
                status: "clean".to_string(),
                orphaned_count: 0,
                removed: vec![],
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("No orphaned containers found.");
        }
        return Ok(());
    }

    if !json {
        println!("Found {} orphaned container(s):", orphans.len());
        for (id, namespace, service) in &orphans {
            let short_id = &id[..12.min(id.len())];
            println!("  - {} ({}/{})", short_id, namespace, service);
        }
    }

    let mut removed = Vec::new();

    if force {
        if !json {
            println!("\nRemoving orphaned containers...");
        }

        for (id, _namespace, _) in &orphans {
            docker
                .remove_container(
                    id,
                    Some(bollard::container::RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await
                .map_err(|e| {
                    crate::error::AetherError::Backend(format!("Remove container failed: {}", e))
                })?;
            let short_id = id[..12.min(id.len())].to_string();
            removed.push(short_id.clone());
            if !json {
                println!("  Removed: {}", short_id);
            }
        }

        if json {
            let output = CleanupOutput {
                status: "cleaned".to_string(),
                orphaned_count: orphans.len(),
                removed,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("Cleanup complete.");
        }
    } else if json {
        let output = CleanupOutput {
            status: "dry_run".to_string(),
            orphaned_count: orphans.len(),
            removed: vec![],
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("\n(Dry run - use --force to actually remove)");
    }

    Ok(())
}
