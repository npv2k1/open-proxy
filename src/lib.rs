//! Open Proxy - Proxy Parser and Checker
//!
//! This is a proxy parser and checker with multi-threading support.
//! It can parse proxies from various formats and check their validity.

pub mod database;
pub mod models;
pub mod proxy;
pub mod tui;

pub use models::*;
pub use proxy::*;

/// Application result type
pub type Result<T> = anyhow::Result<T>;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Database file path
    pub database_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "todo.db".to_string(),
        }
    }
}
