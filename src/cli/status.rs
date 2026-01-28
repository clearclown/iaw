use crate::backend::{Backend, DockerBackend};
use crate::error::Result;
use crate::jj::JjCommand;
use crate::output::json::{ContainerStatus, StatusOutput};
use crate::provisioner::StateManager;
use crate::repo::find_repo_root;
use std::path::Path;

pub async fn handle_status(json: bool) -> Result<()> {
    // 1. Run jj status
    let jj_output = JjCommand::status().execute();
    let jj_status_text = match &jj_output {
        Ok(output) => Some(output.stdout.clone()),
        Err(_) => None,
    };

    // 2. Try to get workspace state
    let repo_root = find_repo_root(Path::new("."));

    if let Ok(root) = repo_root {
        let state_manager = StateManager::new(&root);
        let current_dir = std::env::current_dir()?;
        let workspace_name = current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if let Some(workspace_state) = state_manager.get_workspace(workspace_name)? {
            let backend = DockerBackend::new()?;
            let resources = backend.status(&workspace_state.namespace).await?;

            if json {
                let output = StatusOutput {
                    status: "ok".to_string(),
                    workspace: Some(workspace_name.to_string()),
                    namespace: Some(workspace_state.namespace.clone()),
                    backend: Some(workspace_state.backend_type.clone()),
                    resources: resources
                        .iter()
                        .map(|r| ContainerStatus {
                            service_name: r.service_name.clone(),
                            container_id: r.container_id.clone(),
                            status: r.status.clone(),
                            port_mappings: r.port_mappings.clone(),
                        })
                        .collect(),
                    jj_status: jj_status_text,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                if let Some(text) = &jj_output.ok().map(|o| o.stdout) {
                    print!("{}", text);
                }
                println!("\n=== Infrastructure Status ===");
                println!("Namespace: {}", workspace_state.namespace);
                println!("Backend: {}", workspace_state.backend_type);

                for resource in resources {
                    let short_id = &resource.container_id[..12.min(resource.container_id.len())];
                    println!(
                        "  {} [{}]: {}",
                        resource.service_name, short_id, resource.status
                    );
                    for (internal, external) in &resource.port_mappings {
                        println!("    port {} -> {}", internal, external);
                    }
                }
            }
            return Ok(());
        }
    }

    // No workspace state found
    if json {
        let output = StatusOutput {
            status: "ok".to_string(),
            workspace: None,
            namespace: None,
            backend: None,
            resources: vec![],
            jj_status: jj_status_text,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if let Ok(output) = jj_output {
            print!("{}", output.stdout);
        }
        println!("\n(No Aether infrastructure in current workspace)");
    }

    Ok(())
}
