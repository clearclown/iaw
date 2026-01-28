use crate::error::{AetherError, Result};
use handlebars::Handlebars;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResourceHandle {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>,
}

pub struct ContextInjector {
    handlebars: Handlebars<'static>,
}

impl ContextInjector {
    pub fn new() -> Self {
        Self {
            handlebars: Handlebars::new(),
        }
    }

    pub fn render(
        &self,
        template: &str,
        resources: &HashMap<String, ResourceHandle>,
    ) -> Result<String> {
        let mut services = serde_json::Map::new();

        for (name, resource) in resources {
            let mut ports_map = serde_json::Map::new();
            for (internal, external) in &resource.port_mappings {
                ports_map.insert(internal.to_string(), json!(external));
            }

            services.insert(
                name.clone(),
                json!({
                    "ports": ports_map,
                    "container_id": resource.container_id,
                }),
            );
        }

        let context = json!({ "services": services });

        self.handlebars
            .render_template(template, &context)
            .map_err(|e| AetherError::ContextInjection(e.to_string()))
    }
}

impl Default for ContextInjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_template() {
        let injector = ContextInjector::new();
        let mut resources = HashMap::new();
        let mut port_mappings = HashMap::new();
        port_mappings.insert(5432, 32891);

        resources.insert(
            "postgres".to_string(),
            ResourceHandle {
                service_name: "postgres".to_string(),
                container_id: "abc123".to_string(),
                image: "postgres:15".to_string(),
                port_mappings,
            },
        );

        let template = "DB_PORT={{ services.postgres.ports.5432 }}";
        let result = injector.render(template, &resources).unwrap();
        assert_eq!(result, "DB_PORT=32891");
    }

    #[test]
    fn test_multi_service_template() {
        let injector = ContextInjector::new();
        let mut resources = HashMap::new();

        let mut pg_ports = HashMap::new();
        pg_ports.insert(5432, 32891);
        resources.insert(
            "postgres".to_string(),
            ResourceHandle {
                service_name: "postgres".to_string(),
                container_id: "abc".to_string(),
                image: "postgres:15".to_string(),
                port_mappings: pg_ports,
            },
        );

        let mut redis_ports = HashMap::new();
        redis_ports.insert(6379, 32892);
        resources.insert(
            "redis".to_string(),
            ResourceHandle {
                service_name: "redis".to_string(),
                container_id: "def".to_string(),
                image: "redis:7".to_string(),
                port_mappings: redis_ports,
            },
        );

        let template = r#"DATABASE_URL=postgres://localhost:{{ services.postgres.ports.5432 }}
REDIS_URL=redis://localhost:{{ services.redis.ports.6379 }}"#;

        let result = injector.render(template, &resources).unwrap();
        assert!(result.contains("32891"));
        assert!(result.contains("32892"));
    }
}
