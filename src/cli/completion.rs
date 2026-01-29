use crate::error::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Generate shell completion scripts
pub fn generate_completion(shell: &str, output_dir: Option<&Path>) -> Result<()> {
    let output_path = if let Some(dir) = output_dir {
        dir.join(format!("ajj.{}", shell))
    } else {
        Path::new(format!("ajj.{}", shell).as_str()).to_path_buf()
    };

    match shell {
        "bash" => generate_bash_completion(&output_path)?,
        "zsh" => generate_zsh_completion(&output_path)?,
        "fish" => generate_fish_completion(&output_path)?,
        _ => {
            return Err(crate::error::AetherError::Config(format!(
                "Unsupported shell: {}. Supported: bash, zsh, fish",
                shell
            )))
        }
    }

    println!("Completion script written to: {}", output_path.display());
    print_completion_instructions(shell);
    Ok(())
}

fn generate_bash_completion(path: &Path) -> Result<()> {
    let script = r#"_ajj_completion() {
    local cur prev words cword
    _init_completion || return

    case "${prev}" in
        -c|--config)
            _filedir
            return
            ;;
        -o|--output)
            COMPREPLY=($(compgen -W "human json" -- "${cur}"))
            return
            ;;
        workspace)
            COMPREPLY=($(compgen -W "add forget" -- "${cur}"))
            return
            ;;
        logs|restart|stop|start|exec)
            # Suggest service names from config
            COMPREPLY=($(compgen -W "postgres redis" -- "${cur}"))
            return
            ;;
        cleanup)
            COMPREPLY=($(compgen -W "--force" -- "${cur}"))
            return
            ;;
    esac

    if [[ ${cword} -eq 1 ]]; then
        COMPREPLY=($(compgen -W "workspace run status list cleanup logs restart stop start exec --help" -- "${cur}"))
    fi
}

complete -F _ajj_completion ajj
"#;

    let mut file = File::create(path)?;
    file.write_all(script.as_bytes())?;
    Ok(())
}

fn generate_zsh_completion(path: &Path) -> Result<()> {
    let script = r#"#compdef ajj

_ajj() {
    local -a commands subcommands

    commands=(
        'workspace:Workspace management'
        'run:Run command with workspace environment'
        'status:Show workspace status'
        'list:List all workspaces'
        'cleanup:Cleanup orphaned containers'
        'logs:Show logs from a service'
        'restart:Restart a service'
        'stop:Stop a service'
        'start:Start a service'
        'exec:Execute a command in a service container'
    )

    case $words[2] in
        workspace)
            subcommands=('add:Create new workspace' 'forget:Remove workspace')
            _describe 'command' subcommands
            ;;
        logs|restart|stop|start|exec)
            _services=('postgres' 'redis')
            _describe 'services' _services
            ;;
        *)
            _describe 'command' commands
            ;;
    esac
}

_ajj "$@"
"#;

    let mut file = File::create(path)?;
    file.write_all(script.as_bytes())?;
    Ok(())
}

fn generate_fish_completion(path: &Path) -> Result<()> {
    let script = r#"complete -c ajj -f

complete -c ajj -n __fish_use_subcommand -a workspace -d 'Workspace management'
complete -c ajj -n __fish_use_subcommand -a run -d 'Run command with workspace environment'
complete -c ajj -n __fish_use_subcommand -a status -d 'Show workspace status'
complete -c ajj -n __fish_use_subcommand -a list -d 'List all workspaces'
complete -c ajj -n __fish_use_subcommand -a cleanup -d 'Cleanup orphaned containers'
complete -c ajj -n __fish_use_subcommand -a logs -d 'Show logs from a service'
complete -c ajj -n __fish_use_subcommand -a restart -d 'Restart a service'
complete -c ajj -n __fish_use_subcommand -a stop -d 'Stop a service'
complete -c ajj -n __fish_use_subcommand -a start -d 'Start a service'
complete -c ajj -n __fish_use_subcommand -a exec -d 'Execute a command in a service container'

complete -c ajj -n '__fish_seen_subcommand_from workspace' -a add forget
complete -c ajj -n '__fish_seen_subcommand_from logs restart stop start exec' -a 'postgres redis'
complete -c ajj -n '__fish_seen_subcommand_from cleanup' -l force
complete -c ajj -s o -l output -x -a 'human json'
complete -c ajj -s c -l config -r
"#;

    let mut file = File::create(path)?;
    file.write_all(script.as_bytes())?;
    Ok(())
}

fn print_completion_instructions(shell: &str) {
    println!("\nTo enable completions:");
    match shell {
        "bash" => {
            println!("  # Add to ~/.bashrc:");
            println!("  source ~/$(pwd)/ajj.bash");
        }
        "zsh" => {
            println!("  # Add to ~/.zshrc:");
            println!("  fpath=(~/$(pwd) $fpath)");
            println!("  compinit");
        }
        "fish" => {
            println!("  # Add to ~/.config/fish/completions/ajj.fish");
            println!("  cp ~/$(pwd)/ajj.fish ~/.config/fish/completions/");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_bash_completion() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("ajj.bash");
        let result = generate_bash_completion(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_zsh_completion() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("ajj.zsh");
        let result = generate_zsh_completion(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_fish_completion() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("ajj.fish");
        let result = generate_fish_completion(&path);
        assert!(result.is_ok());
    }
}
