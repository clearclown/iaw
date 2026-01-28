use aether::backend::{Backend, DockerBackend, ServiceSpec};
use std::collections::HashMap;

#[tokio::test]
#[ignore] // Requires Docker daemon
async fn test_docker_provision_deprovision() {
    let backend = DockerBackend::new().unwrap();

    let mut services = HashMap::new();
    let mut port_mappings = HashMap::new();
    port_mappings.insert(80, 8080);

    services.insert(
        "test".to_string(),
        ServiceSpec {
            name: "test".to_string(),
            image: "alpine:latest".to_string(),
            ports: vec![80],
            env: HashMap::new(),
            volumes: vec![],
            command: Some(vec!["sleep".to_string(), "300".to_string()]),
            port_mappings,
        },
    );

    // Provision
    let handles = backend
        .provision("test-namespace-integration", &services)
        .await
        .unwrap();
    assert_eq!(handles.len(), 1);

    // Check status
    let status = backend.status("test-namespace-integration").await.unwrap();
    assert_eq!(status.len(), 1);

    // Deprovision
    backend
        .deprovision("test-namespace-integration")
        .await
        .unwrap();

    // Verify removed
    let status = backend.status("test-namespace-integration").await.unwrap();
    assert_eq!(status.len(), 0);
}

#[test]
fn test_cli_parsing() {
    use aether::cli::Cli;
    use clap::Parser;

    let cli = Cli::parse_from(&["ajj", "workspace", "add", "test"]);
    // Just verify it doesn't panic
    assert!(matches!(
        cli.command,
        aether::cli::Commands::Workspace { .. }
    ));
}

#[test]
fn test_config_loading() {
    use aether::config::AetherConfig;
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("aether.toml");

    let config_content = r#"
[backend]
type = "docker"

[services.test]
image = "alpine:latest"
ports = ["80"]
"#;

    fs::write(&config_path, config_content).unwrap();

    let config: AetherConfig = toml::from_str(config_content).unwrap();
    assert_eq!(config.services.len(), 1);
}

#[test]
fn test_state_management() {
    use aether::provisioner::{StateManager, WorkspaceState};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let manager = StateManager::new(temp_dir.path());

    let workspace = WorkspaceState {
        name: "test".to_string(),
        path: "/tmp/test".to_string(),
        namespace: "aether-test".to_string(),
        backend_type: "docker".to_string(),
        created_at: "2026-01-28T00:00:00Z".to_string(),
        resources: vec![],
    };

    manager.register_workspace(workspace).unwrap();

    let retrieved = manager.get_workspace("test").unwrap();
    assert!(retrieved.is_some());

    manager.unregister_workspace("test").unwrap();
    let removed = manager.get_workspace("test").unwrap();
    assert!(removed.is_none());
}

#[test]
fn test_port_allocator() {
    use aether::provisioner::PortAllocator;

    let allocator = PortAllocator::new();
    let ports = allocator.allocate(10).unwrap();

    assert_eq!(ports.len(), 10);

    // All ports should be unique
    use std::collections::HashSet;
    let unique: HashSet<_> = ports.iter().collect();
    assert_eq!(unique.len(), 10);
}

#[test]
fn test_context_injector() {
    use aether::provisioner::{context_injector::ResourceHandle, ContextInjector};
    use std::collections::HashMap;

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

    let template = "DATABASE_URL=postgres://localhost:{{ services.postgres.ports.5432 }}/db";
    let result = injector.render(template, &resources).unwrap();

    assert!(result.contains("32891"));
}
