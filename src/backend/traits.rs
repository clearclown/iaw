use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Backend: Send + Sync {
    async fn provision(
        &self,
        namespace: &str,
        services: &HashMap<String, ServiceSpec>,
    ) -> Result<Vec<ResourceHandle>>;

    async fn deprovision(&self, namespace: &str) -> Result<()>;

    async fn status(&self, namespace: &str) -> Result<Vec<ResourceStatus>>;

    async fn logs(&self, namespace: &str, service: &str, tail: Option<usize>) -> Result<String>;

    async fn restart(&self, namespace: &str, service: &str) -> Result<()>;

    async fn stop(&self, namespace: &str, service: &str) -> Result<()>;

    async fn start(&self, namespace: &str, service: &str) -> Result<()>;

    async fn run_in_container(
        &self,
        namespace: &str,
        service: &str,
        command: &[String],
    ) -> Result<ContainerExecResult>;

    fn backend_type(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct ServiceSpec {
    pub name: String,
    pub image: String,
    pub ports: Vec<u16>,
    pub env: HashMap<String, String>,
    pub volumes: Vec<String>,
    pub command: Option<Vec<String>>,
    pub port_mappings: HashMap<u16, u16>, // internal -> external
    pub depends_on: Vec<String>,
    pub cpu_limit: Option<f64>,
    pub cpu_reservation: Option<f64>,
    pub memory_limit: Option<i64>,
    pub memory_reservation: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ResourceHandle {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
    pub port_mappings: HashMap<u16, u16>,
}

#[derive(Debug, Clone)]
pub struct ResourceStatus {
    pub service_name: String,
    pub container_id: String,
    pub status: String,
    pub port_mappings: HashMap<u16, u16>,
}

#[derive(Debug, Clone)]
pub struct ContainerExecResult {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}
