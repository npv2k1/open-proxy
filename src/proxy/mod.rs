//! Proxy module for parsing and checking proxies
//!
//! This module provides functionality for:
//! - Parsing proxies from various formats (IP:PORT, IP:PORT:USER:PASS, etc.)
//! - Checking proxy validity with multi-threaded support
//! - Saving good and bad proxies to separate files
//! - Crawling proxy websites to extract proxy lists

pub mod checker;
pub mod crawler;
pub mod models;
pub mod parser;

pub use checker::{CheckerConfig, ProxyChecker};
pub use crawler::{CrawlerConfig, ProxyCrawler, ProxySource};
pub use models::{Proxy, ProxyAuth, ProxyCheckResult, ProxyCheckStatus, ProxyType};
pub use parser::ProxyParser;
