use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AetherConfig {
    pub backend: BackendConfig,
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,
    pub injection: Option<InjectionConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    Docker {
        #[serde(default)]
        socket: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
    pub image: String,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InjectionConfig {
    pub file: String,
    pub template: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let toml_str = r#"
[backend]
type = "docker"

[services.postgres]
image = "postgres:15"
ports = ["5432"]

[injection]
file = ".env"
template = "DB_PORT={{ services.postgres.ports.5432 }}"
"#;

        let config: AetherConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.services.len(), 1);
        assert!(config.injection.is_some());
    }
}
