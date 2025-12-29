use std::time::Duration;

use crossterm::event::{self, KeyEvent, KeyEventKind};

use crate::error::Result;

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Terminal resize event
    Resize,
    /// Tick event for periodic updates
    Tick,
}

/// Event handler for terminal events
pub struct EventHandler {
    /// Tick rate for polling
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with specified tick rate
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next event
    pub fn poll(&self) -> Result<Option<Event>> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                    Ok(Some(Event::Key(key)))
                }
                event::Event::Resize(_, _) => Ok(Some(Event::Resize)),
                _ => Ok(None),
            }
        } else {
            Ok(Some(Event::Tick))
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(16) // ~60fps for smooth animations
    }
}
