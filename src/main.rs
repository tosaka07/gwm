mod app;
mod config;
mod git;
mod hooks;
mod input;
mod ui;

use app::App;
use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use input::{handle_key_event, InputResult};
use ratatui::{backend::CrosstermBackend, Terminal, Viewport};
use std::io::stdout;
use std::path::PathBuf;
use std::process::Command;

/// Git Worktree Manager - A TUI application for managing git worktrees
#[derive(Parser)]
#[command(name = "gwm")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Print selected worktree path to stdout (for shell integration)
    #[arg(short = 'p', long = "print-path")]
    print_path: bool,
}

const INLINE_HEIGHT: u16 = 20;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Load configuration
    let config = config::load_config(cli.config.as_deref()).unwrap_or_default();

    // Initialize git manager
    let git = match git::GitManager::new() {
        Ok(git) => git,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Please run this command from within a git repository.");
            std::process::exit(1);
        }
    };

    // Create application
    let mut app = match App::new(config, git) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error initializing application: {}", e);
            std::process::exit(1);
        }
    };

    // Setup terminal with inline viewport
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout());
    let options = ratatui::TerminalOptions {
        viewport: Viewport::Inline(INLINE_HEIGHT),
    };
    let mut terminal = Terminal::with_options(backend, options)?;

    // Run the application
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;

    // Move cursor below the inline viewport and clear it
    terminal.clear()?;

    // Handle the result
    match result {
        Ok(()) => {
            if let Some(path) = &app.selected_worktree_path {
                if cli.print_path {
                    // Print the path so it can be captured by a shell function
                    println!("{}", path);
                } else {
                    // Launch a subshell in the selected worktree directory
                    launch_subshell(path);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            // Only handle key press events (not release)
            if key.kind == KeyEventKind::Press {
                match handle_key_event(app, key) {
                    InputResult::Quit => break,
                    InputResult::Continue => {}
                }
            }
        }
    }

    Ok(())
}

/// Error type for shell execution
#[derive(Debug)]
enum ShellError {
    /// Shell command failed with exit code
    ExitCode(i32),
    /// Shell command was terminated by signal
    Terminated,
    /// Failed to execute shell
    ExecutionFailed(std::io::Error),
}

/// Launch a subshell in the specified directory
fn launch_subshell(path: &str) {
    let shell = get_shell();
    if let Err(e) = run_shell(&shell, path) {
        match e {
            ShellError::ExitCode(code) => std::process::exit(code),
            ShellError::Terminated => std::process::exit(1),
            ShellError::ExecutionFailed(err) => {
                eprintln!("Failed to launch shell: {}", err);
                std::process::exit(1);
            }
        }
    }
}

/// Run a shell command in the specified directory
/// Returns Ok(()) if shell exits successfully, Err otherwise
fn run_shell(shell: &str, path: &str) -> std::result::Result<(), ShellError> {
    match Command::new(shell).current_dir(path).status() {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else if let Some(code) = status.code() {
                Err(ShellError::ExitCode(code))
            } else {
                Err(ShellError::Terminated)
            }
        }
        Err(e) => Err(ShellError::ExecutionFailed(e)),
    }
}

/// Get the user's shell from SHELL environment variable, fallback to /bin/sh
fn get_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_default() {
        let cli = Cli::parse_from(["gwm"]);
        assert!(cli.config.is_none());
        assert!(!cli.print_path);
    }

    #[test]
    fn test_cli_parse_print_path_short() {
        let cli = Cli::parse_from(["gwm", "-p"]);
        assert!(cli.print_path);
    }

    #[test]
    fn test_cli_parse_print_path_long() {
        let cli = Cli::parse_from(["gwm", "--print-path"]);
        assert!(cli.print_path);
    }

    #[test]
    fn test_cli_parse_config_short() {
        let cli = Cli::parse_from(["gwm", "-c", "/path/to/config.toml"]);
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
    }

    #[test]
    fn test_cli_parse_config_long() {
        let cli = Cli::parse_from(["gwm", "--config", "/path/to/config.toml"]);
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
    }

    #[test]
    fn test_cli_parse_combined() {
        let cli = Cli::parse_from(["gwm", "-p", "-c", "/path/to/config.toml"]);
        assert!(cli.print_path);
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
    }

    #[test]
    fn test_get_shell_returns_shell_env_or_fallback() {
        // Test that get_shell returns either $SHELL or /bin/sh
        let shell = get_shell();
        // Should be a valid path
        assert!(shell.starts_with('/'));
        // Should be either the env var or the fallback
        if let Ok(env_shell) = std::env::var("SHELL") {
            assert_eq!(shell, env_shell);
        } else {
            assert_eq!(shell, "/bin/sh");
        }
    }

    #[test]
    fn test_run_shell_nonexistent_command() {
        let result = run_shell("/nonexistent/shell", "/tmp");
        assert!(result.is_err());
        assert!(matches!(result, Err(ShellError::ExecutionFailed(_))));
    }

    #[test]
    fn test_run_shell_nonexistent_directory() {
        let result = run_shell("/bin/sh", "/nonexistent/directory/path");
        assert!(result.is_err());
        assert!(matches!(result, Err(ShellError::ExecutionFailed(_))));
    }

    #[test]
    fn test_run_shell_exit_code() {
        // Test that non-zero exit codes are properly captured
        // We can't test run_shell directly with exit codes easily,
        // so we verify the underlying Command behavior
        let result = Command::new("/bin/sh").arg("-c").arg("exit 42").status();
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(!status.success());
        assert_eq!(status.code(), Some(42));
    }

    #[test]
    fn test_shell_error_debug() {
        // Test that ShellError implements Debug
        let err = ShellError::ExitCode(1);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ExitCode"));

        let err = ShellError::Terminated;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Terminated"));
    }
}
