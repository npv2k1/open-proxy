//! Proxy module for parsing and checking proxies
//!
//! This module provides functionality for:
//! - Parsing proxies from various formats (IP:PORT, IP:PORT:USER:PASS, etc.)
//! - Checking proxy validity with multi-threaded support
//! - Saving good and bad proxies to separate files

pub mod checker;
pub mod models;
pub mod parser;

pub use checker::{CheckerConfig, ProxyChecker};
pub use models::{Proxy, ProxyAuth, ProxyCheckResult, ProxyCheckStatus, ProxyType};
pub use parser::ProxyParser;
