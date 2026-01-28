use super::schema::AetherConfig;
use crate::error::{AetherError, Result};
use std::path::{Path, PathBuf};

pub fn find_config(start_path: &Path) -> Result<PathBuf> {
    let mut current = start_path.canonicalize()?;

    loop {
        let candidate = current.join("aether.toml");
        if candidate.exists() {
            return Ok(candidate);
        }

        // Stop at repo root (.jj directory)
        if current.join(".jj").is_dir() {
            break;
        }

        current = current
            .parent()
            .ok_or_else(|| AetherError::Config("aether.toml not found".into()))?
            .to_path_buf();
    }

    Err(AetherError::Config("aether.toml not found in repo".into()))
}

pub fn load_config(path: &Path) -> Result<AetherConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AetherError::Config(format!("Failed to read config: {}", e)))?;

    toml::from_str(&content)
        .map_err(|e| AetherError::Config(format!("Failed to parse TOML: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_config_in_current_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("aether.toml");
        fs::write(&config_path, "[backend]\ntype = \"docker\"").unwrap();

        let found = find_config(temp_dir.path()).unwrap();
        assert_eq!(found, config_path);
    }

    #[test]
    fn test_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("aether.toml");
        fs::write(&config_path, "[backend]\ntype = \"docker\"").unwrap();

        let config = load_config(&config_path).unwrap();
        assert!(matches!(
            config.backend,
            super::super::schema::BackendConfig::Docker { .. }
        ));
    }
}
