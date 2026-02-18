mod loader;

pub use loader::load_config_with_sources;
pub use loader::Config;
pub use loader::ConfigError;
pub use loader::ConfigSources;
pub use loader::RepositorySettings;

// Re-export for tests
#[cfg(test)]
pub use loader::UiConfig;
