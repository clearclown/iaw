use crate::error::{AetherError, Result};
use std::process::{Command, Stdio};

pub struct JjCommand {
    args: Vec<String>,
}

pub struct JjOutput {
    pub stdout: String,
    pub stderr: String,
}

impl JjCommand {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    pub fn workspace_add(destination: &str, revision: Option<&str>) -> Self {
        let mut args = vec![
            "workspace".to_string(),
            "add".to_string(),
            destination.to_string(),
        ];

        if let Some(rev) = revision {
            args.push("--revision".to_string());
            args.push(rev.to_string());
        }

        Self { args }
    }

    pub fn workspace_forget(workspace: &str) -> Self {
        Self {
            args: vec![
                "workspace".to_string(),
                "forget".to_string(),
                workspace.to_string(),
            ],
        }
    }

    pub fn status() -> Self {
        Self {
            args: vec!["status".to_string()],
        }
    }

    pub fn execute(&self) -> Result<JjOutput> {
        let output = Command::new("jj")
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    AetherError::Jj {
                        message: "jj command not found. Please install Jujutsu.".to_string(),
                        exit_code: -1,
                    }
                } else {
                    AetherError::Jj {
                        message: format!("Failed to execute jj: {}", e),
                        exit_code: -1,
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_add_args() {
        let cmd = JjCommand::workspace_add("../test-ws", None);
        assert_eq!(cmd.args, vec!["workspace", "add", "../test-ws"]);
    }

    #[test]
    fn test_workspace_add_with_revision() {
        let cmd = JjCommand::workspace_add("../test-ws", Some("main"));
        assert_eq!(
            cmd.args,
            vec!["workspace", "add", "../test-ws", "--revision", "main"]
        );
    }

    #[test]
    fn test_workspace_forget_args() {
        let cmd = JjCommand::workspace_forget("test-ws");
        assert_eq!(cmd.args, vec!["workspace", "forget", "test-ws"]);
    }

    #[test]
    fn test_status_args() {
        let cmd = JjCommand::status();
        assert_eq!(cmd.args, vec!["status"]);
    }
}
