use std::io::{stdout, Stdout};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use crate::error::Result;

pub type CrosstermTerminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

/// Terminal wrapper for managing terminal state
pub struct Terminal {
    terminal: CrosstermTerminal,
}

impl Terminal {
    /// Create a new terminal instance
    pub fn new() -> Result<Self> {
        let terminal = Self::setup()?;
        Ok(Self { terminal })
    }

    /// Setup terminal for TUI
    fn setup() -> Result<CrosstermTerminal> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = ratatui::Terminal::new(backend)?;
        Ok(terminal)
    }

    /// Restore terminal to original state
    fn restore() -> Result<()> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    /// Draw frame using provided closure
    pub fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    /// Suspend terminal for subprocess (restore terminal, run callback, re-setup)
    pub fn suspend<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce() -> T,
    {
        Self::restore()?;
        let result = f();
        self.terminal = Self::setup()?;
        Ok(result)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = Self::restore();
    }
}
