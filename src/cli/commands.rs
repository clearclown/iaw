use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, ValueEnum, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Parser, Debug)]
#[command(name = "ajj")]
#[command(about = "Aether - Infrastructure as Workspace wrapper for Jujutsu", long_about = None)]
pub struct Cli {
    /// Output format
    #[arg(short, long, default_value = "human", global = true)]
    pub output: OutputFormat,

    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn is_json(&self) -> bool {
        self.output == OutputFormat::Json
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Workspace management
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Run command with workspace environment
    Run {
        /// Command to execute
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Show workspace status
    Status,

    /// List all workspaces
    List,

    /// Cleanup orphaned containers
    Cleanup {
        /// Actually remove containers (default is dry-run)
        #[arg(long)]
        force: bool,
    },

    /// Show logs from a service
    Logs {
        /// Service name
        service: String,

        /// Number of lines to show (default: all)
        #[arg(short = 'n', long)]
        tail: Option<usize>,

        /// Follow log output (not yet implemented)
        #[arg(short, long)]
        follow: bool,
    },

    /// Restart a service
    Restart {
        /// Service name
        service: String,
    },

    /// Stop a service (without removing)
    Stop {
        /// Service name
        service: String,
    },

    /// Start a stopped service
    Start {
        /// Service name
        service: String,
    },

    /// Execute a command in a service container
    #[command(name = "exec")]
    ContainerExec {
        /// Service name
        service: String,

        /// Command to execute
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell type (bash, zsh, fish)
        shell: String,

        /// Output directory
        #[arg(short, long)]
        dir: Option<String>,
    },

    /// Pass through to jj command
    #[command(external_subcommand)]
    Jj(Vec<String>),
}

#[derive(Subcommand, Debug)]
pub enum WorkspaceAction {
    /// Create new workspace with infrastructure
    Add {
        /// Destination path
        destination: String,

        /// Revision to checkout
        #[arg(short, long)]
        revision: Option<String>,
    },

    /// Remove workspace and cleanup infrastructure
    Forget {
        /// Workspace name or path
        workspace: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace_add() {
        let cli = Cli::parse_from(["ajj", "workspace", "add", "../test-ws"]);
        match cli.command {
            Commands::Workspace {
                action: WorkspaceAction::Add { destination, .. },
            } => {
                assert_eq!(destination, "../test-ws");
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_run_command() {
        let cli = Cli::parse_from(["ajj", "run", "--", "cargo", "test"]);
        match cli.command {
            Commands::Run { command } => {
                assert_eq!(command, vec!["cargo", "test"]);
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_status() {
        let cli = Cli::parse_from(["ajj", "status"]);
        assert!(matches!(cli.command, Commands::Status));
        assert!(!cli.is_json());
    }

    #[test]
    fn test_parse_output_json() {
        let cli = Cli::parse_from(["ajj", "--output", "json", "status"]);
        assert!(cli.is_json());
    }

    #[test]
    fn test_parse_config_flag() {
        let cli = Cli::parse_from(["ajj", "--config", "/path/to/aether.toml", "status"]);
        assert_eq!(cli.config, Some("/path/to/aether.toml".to_string()));
    }

    #[test]
    fn test_parse_logs_command() {
        let cli = Cli::parse_from(["ajj", "logs", "postgres"]);
        match cli.command {
            Commands::Logs {
                service,
                tail,
                follow,
            } => {
                assert_eq!(service, "postgres");
                assert_eq!(tail, None);
                assert!(!follow);
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_logs_with_tail() {
        let cli = Cli::parse_from(["ajj", "logs", "postgres", "-n", "100"]);
        match cli.command {
            Commands::Logs { service, tail, .. } => {
                assert_eq!(service, "postgres");
                assert_eq!(tail, Some(100));
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_restart_command() {
        let cli = Cli::parse_from(["ajj", "restart", "redis"]);
        match cli.command {
            Commands::Restart { service } => {
                assert_eq!(service, "redis");
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_stop_command() {
        let cli = Cli::parse_from(["ajj", "stop", "postgres"]);
        match cli.command {
            Commands::Stop { service } => {
                assert_eq!(service, "postgres");
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_start_command() {
        let cli = Cli::parse_from(["ajj", "start", "postgres"]);
        match cli.command {
            Commands::Start { service } => {
                assert_eq!(service, "postgres");
            }
            _ => panic!("Wrong command parsed"),
        }
    }

    #[test]
    fn test_parse_exec_command() {
        let cli = Cli::parse_from(["ajj", "exec", "postgres", "--", "psql", "-c", "SELECT 1"]);
        match cli.command {
            Commands::ContainerExec { service, command } => {
                assert_eq!(service, "postgres");
                assert_eq!(command, vec!["psql", "-c", "SELECT 1"]);
            }
            _ => panic!("Wrong command parsed"),
        }
    }
}
