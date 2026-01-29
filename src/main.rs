use aether::cli::{
    generate_completion, handle_cleanup, handle_container_run, handle_list, handle_logs,
    handle_restart, handle_run, handle_start, handle_status, handle_stop, handle_workspace_add,
    handle_workspace_forget,
};
use aether::cli::{Cli, Commands, WorkspaceAction};
use aether::jj::JjCommand;
use clap::Parser;
use std::path::Path;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json = cli.is_json();
    let config_path = cli.config.clone();

    let result = match cli.command {
        Commands::Workspace { action } => match action {
            WorkspaceAction::Add {
                destination,
                revision,
            } => {
                handle_workspace_add(
                    &destination,
                    revision.as_deref(),
                    config_path.as_deref(),
                    json,
                )
                .await
            }
            WorkspaceAction::Forget { workspace } => {
                handle_workspace_forget(&workspace, json).await
            }
        },
        Commands::Run { command } => match handle_run(&command) {
            Ok(exit_code) => std::process::exit(exit_code),
            Err(e) => {
                if json {
                    let err = serde_json::json!({
                        "status": "error",
                        "error": e.to_string()
                    });
                    eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        },
        Commands::Status => handle_status(json).await,
        Commands::List => handle_list(json).await,
        Commands::Cleanup { force } => handle_cleanup(force, json).await,
        Commands::Logs {
            service,
            tail,
            follow: _,
        } => handle_logs(&service, tail, json).await,
        Commands::Restart { service } => handle_restart(&service, json).await,
        Commands::Stop { service } => handle_stop(&service, json).await,
        Commands::Start { service } => handle_start(&service, json).await,
        Commands::ContainerExec { service, command } => {
            handle_container_run(&service, &command, json).await
        }
        Commands::Completion { shell, dir } => {
            let dir_path = dir.as_deref().map(Path::new);
            generate_completion(&shell, dir_path)
        }
        Commands::Jj(args) => {
            let cmd = JjCommand::new(args);
            match cmd.execute() {
                Ok(output) => {
                    print!("{}", output.stdout);
                    eprint!("{}", output.stderr);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    };

    if let Err(e) = result {
        if json {
            let err = serde_json::json!({
                "status": "error",
                "error": e.to_string()
            });
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
        } else {
            eprintln!("Error: {}", e);
        }
        std::process::exit(1);
    }
}
