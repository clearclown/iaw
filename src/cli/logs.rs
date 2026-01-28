use crate::backend::{Backend, DockerBackend};
use crate::error::Result;
use crate::provisioner::{StateManager, WorkspaceState};
use crate::repo::find_repo_root;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct LogsOutput {
    pub status: String,
    pub service: String,
    pub logs: String,
}

#[derive(Debug, Serialize)]
pub struct ServiceActionOutput {
    pub status: String,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContainerRunOutput {
    pub status: String,
    pub service: String,
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

/// Get the current workspace state from the current directory
fn get_current_workspace() -> Result<(String, WorkspaceState)> {
    let repo_root = find_repo_root(Path::new("."))?;
    let state_manager = StateManager::new(&repo_root);

    let current_dir = std::env::current_dir()?;
    let workspace_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let workspace_state = state_manager
        .get_workspace(&workspace_name)?
        .ok_or_else(|| {
            crate::error::AetherError::State(format!(
                "Workspace '{}' not found. Are you in an Aether-managed workspace?",
                workspace_name
            ))
        })?;

    Ok((workspace_name, workspace_state))
}

pub async fn handle_logs(service: &str, tail: Option<usize>, json: bool) -> Result<()> {
    let (_workspace_name, workspace_state) = get_current_workspace()?;

    let backend = DockerBackend::new()?;
    let logs = backend
        .logs(&workspace_state.namespace, service, tail)
        .await?;

    if json {
        let output = LogsOutput {
            status: "ok".to_string(),
            service: service.to_string(),
            logs,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if logs.is_empty() {
        println!("No logs available for service '{}'", service);
    } else {
        print!("{}", logs);
    }

    Ok(())
}

pub async fn handle_restart(service: &str, json: bool) -> Result<()> {
    let (_workspace_name, workspace_state) = get_current_workspace()?;

    let backend = DockerBackend::new()?;
    backend.restart(&workspace_state.namespace, service).await?;

    if json {
        let output = ServiceActionOutput {
            status: "ok".to_string(),
            service: service.to_string(),
            message: Some("restarted".to_string()),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Service '{}' restarted successfully", service);
    }

    Ok(())
}

pub async fn handle_stop(service: &str, json: bool) -> Result<()> {
    let (_workspace_name, workspace_state) = get_current_workspace()?;

    let backend = DockerBackend::new()?;
    backend.stop(&workspace_state.namespace, service).await?;

    if json {
        let output = ServiceActionOutput {
            status: "ok".to_string(),
            service: service.to_string(),
            message: Some("stopped".to_string()),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Service '{}' stopped", service);
    }

    Ok(())
}

pub async fn handle_start(service: &str, json: bool) -> Result<()> {
    let (_workspace_name, workspace_state) = get_current_workspace()?;

    let backend = DockerBackend::new()?;
    backend.start(&workspace_state.namespace, service).await?;

    if json {
        let output = ServiceActionOutput {
            status: "ok".to_string(),
            service: service.to_string(),
            message: Some("started".to_string()),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Service '{}' started", service);
    }

    Ok(())
}

pub async fn handle_container_run(service: &str, command: &[String], json: bool) -> Result<()> {
    let (_workspace_name, workspace_state) = get_current_workspace()?;

    let backend = DockerBackend::new()?;
    let result = backend
        .run_in_container(&workspace_state.namespace, service, command)
        .await?;

    if json {
        let output = ContainerRunOutput {
            status: if result.exit_code == 0 { "ok" } else { "error" }.to_string(),
            service: service.to_string(),
            exit_code: result.exit_code,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print!("{}", result.stdout);
        if !result.stderr.is_empty() {
            eprint!("{}", result.stderr);
        }
    }

    if result.exit_code != 0 {
        std::process::exit(result.exit_code as i32);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_output_serialization() {
        let output = LogsOutput {
            status: "ok".to_string(),
            service: "postgres".to_string(),
            logs: "Starting PostgreSQL...".to_string(),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("postgres"));
        assert!(json.contains("Starting PostgreSQL"));
    }
}
