mod loader;

pub use loader::load_config;
pub use loader::Config;
pub use loader::ConfigError;
pub use loader::RepositorySettings;

// Re-export for tests
#[cfg(test)]
pub use loader::UiConfig;
