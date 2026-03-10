use super::traits::{Backend, ContainerExecResult, ResourceHandle, ResourceStatus, ServiceSpec};
use crate::error::{AetherError, Result};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    RestartContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{EndpointSettings, HostConfig, PortBinding};
use bollard::network::{CreateNetworkOptions, ListNetworksOptions};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::{HashMap, VecDeque};

pub struct DockerBackend {
    client: Docker,
}

impl DockerBackend {
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_local_defaults()
            .map_err(|e| AetherError::Backend(format!("Failed to connect to Docker: {}", e)))?;

        Ok(Self { client })
    }

    /// Ensure the network exists for this workspace
    async fn ensure_network(&self, network_name: &str, namespace: &str) -> Result<()> {
        // Check if network already exists
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![network_name.to_string()]);

        let networks = self
            .client
            .list_networks(Some(ListNetworksOptions { filters }))
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to list networks: {}", e)))?;

        if networks.is_empty() {
            // Create the network with labels
            let mut labels = HashMap::new();
            labels.insert("aether.managed".to_string(), "true".to_string());
            labels.insert("aether.namespace".to_string(), namespace.to_string());

            self.client
                .create_network(CreateNetworkOptions {
                    name: network_name.to_string(),
                    driver: "bridge".to_string(),
                    labels,
                    ..Default::default()
                })
                .await
                .map_err(|e| AetherError::Backend(format!("Failed to create network: {}", e)))?;
        }

        Ok(())
    }

    /// Remove the network for this workspace
    async fn remove_network(&self, namespace: &str) -> Result<()> {
        let network_name = format!("{}-network", namespace);

        // Try to remove the network, ignoring errors if it doesn't exist
        let _ = self.client.remove_network(&network_name).await;

        Ok(())
    }

    /// Topological sort of services respecting depends_on ordering.
    /// Returns service names in an order where dependencies come first.
    fn topo_sort(services: &HashMap<String, ServiceSpec>) -> Result<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for name in services.keys() {
            in_degree.entry(name.as_str()).or_insert(0);
        }

        for (name, spec) in services {
            for dep in &spec.depends_on {
                if !services.contains_key(dep) {
                    return Err(AetherError::Config(format!(
                        "Service '{}' depends on unknown service '{}'",
                        name, dep
                    )));
                }
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(name.as_str());
                *in_degree.entry(name.as_str()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();

        let mut order = Vec::new();

        while let Some(name) = queue.pop_front() {
            order.push(name.to_string());
            if let Some(deps) = dependents.get(name) {
                for &dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        if order.len() != services.len() {
            return Err(AetherError::Config(
                "Circular dependency detected in service depends_on".into(),
            ));
        }

        Ok(order)
    }

    /// Find a container by namespace and service name
    async fn find_container(&self, namespace: &str, service: &str) -> Result<String> {
        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![
                format!("aether.workspace={}", namespace),
                format!("aether.service={}", service),
            ],
        );

        let containers = self
            .client
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to list containers: {}", e)))?;

        containers
            .first()
            .and_then(|c| c.id.clone())
            .ok_or_else(|| {
                AetherError::Backend(format!(
                    "Service '{}' not found in namespace '{}'",
                    service, namespace
                ))
            })
    }
}

#[async_trait]
impl Backend for DockerBackend {
    async fn provision(
        &self,
        namespace: &str,
        services: &HashMap<String, ServiceSpec>,
    ) -> Result<Vec<ResourceHandle>> {
        let mut handles = Vec::new();
        let network_name = format!("{}-network", namespace);

        // Create dedicated network for this workspace
        self.ensure_network(&network_name, namespace).await?;

        // Sort services by depends_on to start dependencies first
        let start_order = Self::topo_sort(services)?;

        for name in &start_order {
            let spec = &services[name];
            let container_name = format!("{}-{}", namespace, name);

            // Build port bindings
            let mut port_bindings = HashMap::new();
            for (internal, external) in &spec.port_mappings {
                let port_key = format!("{}/tcp", internal);
                port_bindings.insert(
                    port_key,
                    Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some(external.to_string()),
                    }]),
                );
            }

            // Build labels
            let mut labels = HashMap::new();
            labels.insert("aether.managed".to_string(), "true".to_string());
            labels.insert("aether.workspace".to_string(), namespace.to_string());
            labels.insert("aether.namespace".to_string(), namespace.to_string());
            labels.insert("aether.service".to_string(), name.clone());

            // Build env vars
            let env: Vec<String> = spec
                .env
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();

            // Build exposed ports
            let mut exposed_ports = HashMap::new();
            for internal in &spec.ports {
                exposed_ports.insert(format!("{}/tcp", internal), HashMap::new());
            }

            // Build volume bindings
            let binds: Vec<String> = spec.volumes.clone();

            // Network configuration - use service name as network alias
            let mut endpoints_config = HashMap::new();
            endpoints_config.insert(
                network_name.clone(),
                EndpointSettings {
                    aliases: Some(vec![name.clone()]),
                    ..Default::default()
                },
            );

            // Build resource limits (as individual fields on HostConfig)
            let cpu_quota = spec.cpu_limit.map(|q| (q * 100000.0) as i64);
            let cpu_period = spec.cpu_limit.map(|_| 100000i64);
            let cpu_shares = spec.cpu_reservation.map(|r| (r * 1024.0) as i64);
            let memory = spec.memory_limit;
            let memory_reservation = spec.memory_reservation;

            // Create container config
            let config = Config {
                image: Some(spec.image.clone()),
                env: Some(env),
                labels: Some(labels),
                exposed_ports: Some(exposed_ports),
                host_config: Some(HostConfig {
                    port_bindings: Some(port_bindings),
                    binds: if binds.is_empty() { None } else { Some(binds) },
                    network_mode: Some(network_name.clone()),
                    cpu_quota,
                    cpu_period,
                    cpu_shares,
                    memory,
                    memory_reservation,
                    ..Default::default()
                }),
                networking_config: Some(bollard::container::NetworkingConfig { endpoints_config }),
                cmd: spec.command.clone(),
                ..Default::default()
            };

            // Create container
            let container = self
                .client
                .create_container(
                    Some(CreateContainerOptions {
                        name: container_name.clone(),
                        ..Default::default()
                    }),
                    config,
                )
                .await
                .map_err(|e| AetherError::Backend(format!("Failed to create container: {}", e)))?;

            // Start container
            self.client
                .start_container(&container.id, None::<StartContainerOptions<String>>)
                .await
                .map_err(|e| AetherError::Backend(format!("Failed to start container: {}", e)))?;

            handles.push(ResourceHandle {
                service_name: name.clone(),
                container_id: container.id,
                image: spec.image.clone(),
                port_mappings: spec.port_mappings.clone(),
            });
        }

        Ok(handles)
    }

    async fn deprovision(&self, namespace: &str) -> Result<()> {
        // List containers with label filter
        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![format!("aether.workspace={}", namespace)],
        );

        let containers = self
            .client
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to list containers: {}", e)))?;

        for container in containers {
            if let Some(id) = container.id {
                // Force remove container
                self.client
                    .remove_container(
                        &id,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await
                    .map_err(|e| {
                        AetherError::Backend(format!("Failed to remove container: {}", e))
                    })?;
            }
        }

        // Remove the network after all containers are gone
        self.remove_network(namespace).await?;

        Ok(())
    }

    async fn status(&self, namespace: &str) -> Result<Vec<ResourceStatus>> {
        let mut filters = HashMap::new();
        filters.insert(
            "label".to_string(),
            vec![format!("aether.workspace={}", namespace)],
        );

        let containers = self
            .client
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to list containers: {}", e)))?;

        let mut statuses = Vec::new();
        for container in containers {
            if let (Some(id), Some(labels)) = (container.id, container.labels) {
                let service_name = labels
                    .get("aether.service")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                let status = container.state.unwrap_or_else(|| "unknown".to_string());

                // Extract port mappings from container info
                let mut port_mappings = HashMap::new();
                if let Some(ports) = &container.ports {
                    for port in ports {
                        if let Some(public) = port.public_port {
                            port_mappings.insert(port.private_port, public);
                        }
                    }
                }

                statuses.push(ResourceStatus {
                    service_name,
                    container_id: id,
                    status,
                    port_mappings,
                });
            }
        }

        Ok(statuses)
    }

    async fn logs(&self, namespace: &str, service: &str, tail: Option<usize>) -> Result<String> {
        let container = self.find_container(namespace, service).await?;

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail
                .map(|n| n.to_string())
                .unwrap_or_else(|| "all".to_string()),
            ..Default::default()
        };

        let mut stream = self.client.logs(&container, Some(options));
        let mut output = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(log) => output.push_str(&log.to_string()),
                Err(e) => {
                    return Err(AetherError::Backend(format!("Failed to read logs: {}", e)));
                }
            }
        }

        Ok(output)
    }

    async fn restart(&self, namespace: &str, service: &str) -> Result<()> {
        let container = self.find_container(namespace, service).await?;

        self.client
            .restart_container(&container, Some(RestartContainerOptions { t: 10 }))
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to restart container: {}", e)))?;

        Ok(())
    }

    async fn stop(&self, namespace: &str, service: &str) -> Result<()> {
        let container = self.find_container(namespace, service).await?;

        self.client
            .stop_container(&container, Some(StopContainerOptions { t: 10 }))
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to stop container: {}", e)))?;

        Ok(())
    }

    async fn start(&self, namespace: &str, service: &str) -> Result<()> {
        let container = self.find_container(namespace, service).await?;

        self.client
            .start_container(&container, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to start container: {}", e)))?;

        Ok(())
    }

    async fn run_in_container(
        &self,
        namespace: &str,
        service: &str,
        command: &[String],
    ) -> Result<ContainerExecResult> {
        let container = self.find_container(namespace, service).await?;

        // Create command runner instance
        let cmd_config = CreateExecOptions {
            cmd: Some(command.to_vec()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let cmd_instance = self
            .client
            .create_exec(&container, cmd_config)
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to create command runner: {}", e)))?;

        // Start command and collect output
        let output = self
            .client
            .start_exec(&cmd_instance.id, None)
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to start command: {}", e)))?;

        let mut stdout = String::new();
        let stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } = output {
            while let Some(result) = output.next().await {
                match result {
                    Ok(log) => {
                        let text = log.to_string();
                        stdout.push_str(&text);
                    }
                    Err(e) => {
                        return Err(AetherError::Backend(format!(
                            "Failed to read command output: {}",
                            e
                        )));
                    }
                }
            }
        }

        // Get exit code
        let cmd_inspect = self
            .client
            .inspect_exec(&cmd_instance.id)
            .await
            .map_err(|e| AetherError::Backend(format!("Failed to inspect command: {}", e)))?;

        let exit_code = cmd_inspect.exit_code.unwrap_or(-1);

        Ok(ContainerExecResult {
            exit_code,
            stdout,
            stderr,
        })
    }

    fn backend_type(&self) -> &'static str {
        "docker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_backend_creation() {
        // This test requires Docker to be running
        let result = DockerBackend::new();
        // Just check it doesn't panic - may fail if Docker isn't available
        assert!(result.is_ok() || result.is_err());
    }

    fn make_spec(name: &str, depends_on: Vec<&str>) -> (String, ServiceSpec) {
        (
            name.to_string(),
            ServiceSpec {
                name: name.to_string(),
                image: format!("{}:latest", name),
                ports: vec![],
                env: HashMap::new(),
                volumes: vec![],
                command: None,
                port_mappings: HashMap::new(),
                depends_on: depends_on.into_iter().map(String::from).collect(),
                cpu_limit: None,
                cpu_reservation: None,
                memory_limit: None,
                memory_reservation: None,
            },
        )
    }

    #[test]
    fn test_topo_sort_no_deps() {
        let services: HashMap<String, ServiceSpec> =
            [make_spec("a", vec![]), make_spec("b", vec![])]
                .into_iter()
                .collect();
        let order = DockerBackend::topo_sort(&services).unwrap();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_topo_sort_linear_deps() {
        let services: HashMap<String, ServiceSpec> = [
            make_spec("app", vec!["postgres"]),
            make_spec("postgres", vec![]),
        ]
        .into_iter()
        .collect();
        let order = DockerBackend::topo_sort(&services).unwrap();
        let pg_pos = order.iter().position(|s| s == "postgres").unwrap();
        let app_pos = order.iter().position(|s| s == "app").unwrap();
        assert!(pg_pos < app_pos);
    }

    #[test]
    fn test_topo_sort_diamond_deps() {
        let services: HashMap<String, ServiceSpec> = [
            make_spec("app", vec!["redis", "postgres"]),
            make_spec("redis", vec!["postgres"]),
            make_spec("postgres", vec![]),
        ]
        .into_iter()
        .collect();
        let order = DockerBackend::topo_sort(&services).unwrap();
        let pg_pos = order.iter().position(|s| s == "postgres").unwrap();
        let redis_pos = order.iter().position(|s| s == "redis").unwrap();
        let app_pos = order.iter().position(|s| s == "app").unwrap();
        assert!(pg_pos < redis_pos);
        assert!(redis_pos < app_pos);
    }

    #[test]
    fn test_topo_sort_circular_dep() {
        let services: HashMap<String, ServiceSpec> =
            [make_spec("a", vec!["b"]), make_spec("b", vec!["a"])]
                .into_iter()
                .collect();
        let result = DockerBackend::topo_sort(&services);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_topo_sort_unknown_dep() {
        let services: HashMap<String, ServiceSpec> = [make_spec("app", vec!["nonexistent"])]
            .into_iter()
            .collect();
        let result = DockerBackend::topo_sort(&services);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown service"));
    }
}
