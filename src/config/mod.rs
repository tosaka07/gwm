mod default;
mod keybinding;
mod loader;
mod types;

pub use default::default_bindings;
pub use keybinding::{parse_key, parse_modifiers};
pub use loader::ConfigLoader;
pub use types::*;

use crate::error::Result;

impl Config {
    /// Load configuration from global and local config files
    pub fn load() -> Result<Self> {
        ConfigLoader::load()
    }
}
