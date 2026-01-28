use crate::error::{AetherError, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceRegistry {
    pub version: String,
    pub workspaces: HashMap<String, WorkspaceState>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceState {
    pub name: String,
    pub path: String,
    pub namespace: String,
    pub backend_type: String,
    pub created_at: String,
    pub resources: Vec<ResourceInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceInfo {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>,
}

pub struct StateManager {
    state_file: PathBuf,
}

impl StateManager {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            state_file: repo_root.join(".aether/state.json"),
        }
    }

    fn acquire_lock(&self) -> Result<File> {
        let lock_path = self.state_file.with_extension("lock");
        std::fs::create_dir_all(lock_path.parent().unwrap())?;

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_path)?;
        file.lock_exclusive()
            .map_err(|e| AetherError::State(format!("Failed to acquire lock: {}", e)))?;
        Ok(file)
    }

    fn load_registry(&self) -> Result<WorkspaceRegistry> {
        if !self.state_file.exists() {
            return Ok(WorkspaceRegistry {
                version: "1.0".to_string(),
                workspaces: HashMap::new(),
            });
        }

        let content = std::fs::read_to_string(&self.state_file)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn atomic_write(&self, registry: &WorkspaceRegistry) -> Result<()> {
        std::fs::create_dir_all(self.state_file.parent().unwrap())?;

        let tmp_path = self.state_file.with_extension("tmp");
        let json = serde_json::to_string_pretty(registry)?;
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(tmp_path, &self.state_file)?;
        Ok(())
    }

    pub fn register_workspace(&self, workspace: WorkspaceState) -> Result<()> {
        let _lock = self.acquire_lock()?;
        let mut registry = self.load_registry()?;
        registry
            .workspaces
            .insert(workspace.name.clone(), workspace);
        self.atomic_write(&registry)?;
        Ok(())
    }

    pub fn unregister_workspace(&self, name: &str) -> Result<()> {
        let _lock = self.acquire_lock()?;
        let mut registry = self.load_registry()?;
        registry.workspaces.remove(name);
        self.atomic_write(&registry)?;
        Ok(())
    }

    pub fn get_workspace(&self, name: &str) -> Result<Option<WorkspaceState>> {
        let _lock = self.acquire_lock()?;
        let registry = self.load_registry()?;
        Ok(registry.workspaces.get(name).cloned())
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceState>> {
        let _lock = self.acquire_lock()?;
        let registry = self.load_registry()?;
        Ok(registry.workspaces.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_register_and_get_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path());

        let workspace = WorkspaceState {
            name: "test-ws".to_string(),
            path: "/tmp/test".to_string(),
            namespace: "aether-test".to_string(),
            backend_type: "docker".to_string(),
            created_at: "2026-01-28T00:00:00Z".to_string(),
            resources: vec![],
        };

        manager.register_workspace(workspace).unwrap();

        let retrieved = manager.get_workspace("test-ws").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-ws");
    }

    #[test]
    fn test_list_workspaces() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path());

        let workspaces = manager.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 0);
    }
}
