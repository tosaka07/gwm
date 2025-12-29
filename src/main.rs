mod action;
mod app;
mod config;
mod error;
mod git;
mod tui;

use color_eyre::Result;

use crate::app::App;
use crate::config::Config;
use crate::tui::Terminal;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Load configuration
    let config = Config::load()?;

    // Initialize terminal
    let terminal = Terminal::new()?;

    // Create and run application
    let mut app = App::new(config)?;
    app.run(terminal).await?;

    Ok(())
}
