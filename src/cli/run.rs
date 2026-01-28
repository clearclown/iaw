use crate::error::Result;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn handle_run(command_args: &[String]) -> Result<i32> {
    if command_args.is_empty() {
        return Err(crate::error::AetherError::Config(
            "No command provided".into(),
        ));
    }

    // 1. Load .env file from current directory
    let env_vars = load_env_file(".env")?;

    // 2. Merge with system environment
    let mut child = Command::new(&command_args[0]);
    child.args(&command_args[1..]);

    for (key, value) in env_vars {
        child.env(key, value);
    }

    // 3. Execute with stdio inheritance
    child.stdin(Stdio::inherit());
    child.stdout(Stdio::inherit());
    child.stderr(Stdio::inherit());

    let status = child.status()?;

    Ok(status.code().unwrap_or(1))
}

fn load_env_file(path: &str) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    if !Path::new(path).exists() {
        return Ok(env_vars); // No .env file is okay
    }

    let content = std::fs::read_to_string(path)?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            env_vars.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(env_vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_env_file_nonexistent() {
        let env = load_env_file("nonexistent.env").unwrap();
        assert_eq!(env.len(), 0);
    }

    #[test]
    fn test_load_env_file_parsing() {
        use std::fs;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        fs::write(&temp_file, "KEY1=value1\nKEY2=value2\n# comment\n").unwrap();

        let env = load_env_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env.len(), 2);
    }
}
