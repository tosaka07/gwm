mod app;
mod config;
mod git;
mod hooks;
mod input;
mod ui;

use app::App;
use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use input::{handle_key_event, InputResult};
use ratatui::{backend::CrosstermBackend, Terminal, Viewport};
use std::io::stdout;

const INLINE_HEIGHT: u16 = 20;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Load configuration
    let config = config::load_config().unwrap_or_default();

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
            // If a worktree was selected, print the cd command
            if let Some(path) = &app.selected_worktree_path {
                // Print the path so it can be captured by a shell function
                println!("{}", path);
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
