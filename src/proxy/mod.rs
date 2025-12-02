//! Proxy module for parsing and checking proxies
//!
//! This module provides functionality for:
//! - Parsing proxies from various formats (IP:PORT, IP:PORT:USER:PASS, etc.)
//! - Checking proxy validity with multi-threaded support
//! - Saving good and bad proxies to separate files
//! - Detecting proxy IP geolocation using MMDB databases

pub mod checker;
pub mod geo;
pub mod models;
pub mod parser;

pub use checker::{CheckerConfig, ProxyChecker};
pub use geo::{GeoLocation, GeoLocator};
pub use models::{Proxy, ProxyAuth, ProxyCheckResult, ProxyCheckStatus, ProxyType};
pub use parser::ProxyParser;
