//! Proxy data models

use serde::{Deserialize, Serialize};
use std::fmt;

/// Proxy type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyType {
    #[default]
    Http,
    Https,
    Socks4,
    Socks5,
}

impl fmt::Display for ProxyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyType::Http => write!(f, "http"),
            ProxyType::Https => write!(f, "https"),
            ProxyType::Socks4 => write!(f, "socks4"),
            ProxyType::Socks5 => write!(f, "socks5"),
        }
    }
}

/// Proxy authentication credentials
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

impl ProxyAuth {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

/// Proxy model representing a single proxy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
    pub proxy_type: ProxyType,
    pub auth: Option<ProxyAuth>,
}

impl Proxy {
    /// Create a new proxy without authentication
    pub fn new(host: String, port: u16, proxy_type: ProxyType) -> Self {
        Self {
            host,
            port,
            proxy_type,
            auth: None,
        }
    }

    /// Create a new proxy with authentication
    pub fn with_auth(
        host: String,
        port: u16,
        proxy_type: ProxyType,
        username: String,
        password: String,
    ) -> Self {
        Self {
            host,
            port,
            proxy_type,
            auth: Some(ProxyAuth::new(username, password)),
        }
    }

    /// Get the proxy URL string
    pub fn url(&self) -> String {
        let auth_part = self.auth.as_ref().map_or(String::new(), |auth| {
            format!("{}:{}@", auth.username, auth.password)
        });

        format!("{}://{}{}:{}", self.proxy_type, auth_part, self.host, self.port)
    }

    /// Get the proxy string in IP:PORT format
    pub fn to_simple_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the proxy string with auth in IP:PORT:USER:PASS format
    pub fn to_full_string(&self) -> String {
        match &self.auth {
            Some(auth) => format!("{}:{}:{}:{}", self.host, self.port, auth.username, auth.password),
            None => self.to_simple_string(),
        }
    }
}

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.url())
    }
}

/// Result of proxy check operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyCheckStatus {
    Working,
    Failed(String),
    Timeout,
}

/// Detailed result of a proxy check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCheckResult {
    pub proxy: Proxy,
    pub status: ProxyCheckStatus,
    pub response_time_ms: Option<u64>,
}

impl ProxyCheckResult {
    pub fn working(proxy: Proxy, response_time_ms: u64) -> Self {
        Self {
            proxy,
            status: ProxyCheckStatus::Working,
            response_time_ms: Some(response_time_ms),
        }
    }

    pub fn failed(proxy: Proxy, error: String) -> Self {
        Self {
            proxy,
            status: ProxyCheckStatus::Failed(error),
            response_time_ms: None,
        }
    }

    pub fn timeout(proxy: Proxy) -> Self {
        Self {
            proxy,
            status: ProxyCheckStatus::Timeout,
            response_time_ms: None,
        }
    }

    pub fn is_working(&self) -> bool {
        matches!(self.status, ProxyCheckStatus::Working)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_creation() {
        let proxy = Proxy::new("127.0.0.1".to_string(), 8080, ProxyType::Http);
        assert_eq!(proxy.host, "127.0.0.1");
        assert_eq!(proxy.port, 8080);
        assert_eq!(proxy.proxy_type, ProxyType::Http);
        assert!(proxy.auth.is_none());
    }

    #[test]
    fn test_proxy_with_auth() {
        let proxy = Proxy::with_auth(
            "127.0.0.1".to_string(),
            8080,
            ProxyType::Socks5,
            "user".to_string(),
            "pass".to_string(),
        );
        assert!(proxy.auth.is_some());
        let auth = proxy.auth.unwrap();
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn test_proxy_url() {
        let proxy = Proxy::new("127.0.0.1".to_string(), 8080, ProxyType::Http);
        assert_eq!(proxy.url(), "http://127.0.0.1:8080");

        let proxy_with_auth = Proxy::with_auth(
            "192.168.1.1".to_string(),
            1080,
            ProxyType::Socks5,
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(proxy_with_auth.url(), "socks5://user:pass@192.168.1.1:1080");
    }

    #[test]
    fn test_proxy_simple_string() {
        let proxy = Proxy::new("127.0.0.1".to_string(), 8080, ProxyType::Http);
        assert_eq!(proxy.to_simple_string(), "127.0.0.1:8080");
    }

    #[test]
    fn test_proxy_full_string() {
        let proxy = Proxy::with_auth(
            "127.0.0.1".to_string(),
            8080,
            ProxyType::Http,
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(proxy.to_full_string(), "127.0.0.1:8080:user:pass");
    }

    #[test]
    fn test_proxy_check_result() {
        let proxy = Proxy::new("127.0.0.1".to_string(), 8080, ProxyType::Http);
        
        let result = ProxyCheckResult::working(proxy.clone(), 100);
        assert!(result.is_working());
        assert_eq!(result.response_time_ms, Some(100));

        let result = ProxyCheckResult::failed(proxy.clone(), "Connection refused".to_string());
        assert!(!result.is_working());

        let result = ProxyCheckResult::timeout(proxy);
        assert!(!result.is_working());
    }
}
