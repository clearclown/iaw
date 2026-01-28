use crate::error::AetherError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct AjjOutput {
    pub status: String,
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub root: String,
    pub backend: String,
    pub namespace: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourceDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceDetail {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusOutput {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ContainerStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jj_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerStatus {
    pub service_name: String,
    pub container_id: String,
    pub status: String,
    pub port_mappings: HashMap<u16, u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupOutput {
    pub status: String,
    pub orphaned_count: usize,
    pub removed: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

impl From<AetherError> for ErrorInfo {
    fn from(err: AetherError) -> Self {
        match err {
            AetherError::Jj { message, exit_code } => ErrorInfo {
                code: format!("JJ_FAILED_{}", exit_code),
                message,
            },
            AetherError::Config(msg) => ErrorInfo {
                code: "CONFIG_ERROR".to_string(),
                message: msg,
            },
            AetherError::Backend(msg) => ErrorInfo {
                code: "BACKEND_ERROR".to_string(),
                message: msg,
            },
            AetherError::PortAllocation(msg) => ErrorInfo {
                code: "PORT_ALLOCATION_ERROR".to_string(),
                message: msg,
            },
            AetherError::ContextInjection(msg) => ErrorInfo {
                code: "CONTEXT_INJECTION_ERROR".to_string(),
                message: msg,
            },
            AetherError::State(msg) => ErrorInfo {
                code: "STATE_ERROR".to_string(),
                message: msg,
            },
            _ => ErrorInfo {
                code: "UNKNOWN_ERROR".to_string(),
                message: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_info_from_config_error() {
        let err = AetherError::Config("test config error".to_string());
        let info: ErrorInfo = err.into();
        assert_eq!(info.code, "CONFIG_ERROR");
        assert_eq!(info.message, "test config error");
    }

    #[test]
    fn test_error_info_from_jj_error() {
        let err = AetherError::Jj {
            message: "jj failed".to_string(),
            exit_code: 1,
        };
        let info: ErrorInfo = err.into();
        assert_eq!(info.code, "JJ_FAILED_1");
    }

    #[test]
    fn test_ajj_output_serialization() {
        let output = AjjOutput {
            status: "ready".to_string(),
            operation: "workspace_add".to_string(),
            workspace: Some(WorkspaceInfo {
                name: "feature-x".to_string(),
                root: "/tmp/feature-x".to_string(),
                backend: "docker".to_string(),
                namespace: "aether-feature-x".to_string(),
                resources: vec![],
            }),
            errors: vec![],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"status\":\"ready\""));
        assert!(json.contains("\"operation\":\"workspace_add\""));
    }

    #[test]
    fn test_status_output_serialization() {
        let output = StatusOutput {
            status: "ok".to_string(),
            workspace: Some("feature-x".to_string()),
            namespace: Some("aether-feature-x".to_string()),
            backend: Some("docker".to_string()),
            resources: vec![ContainerStatus {
                service_name: "postgres".to_string(),
                container_id: "abc123".to_string(),
                status: "running".to_string(),
                port_mappings: HashMap::from([(5432, 32891)]),
            }],
            jj_status: None,
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("postgres"));
        assert!(json.contains("32891"));
    }
}
