//! Proxy parser module for parsing proxies from various formats

use crate::proxy::models::{Proxy, ProxyType};
use crate::Result;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Proxy parser for parsing proxies from strings and files
pub struct ProxyParser;

impl ProxyParser {
    /// Parse a single proxy line
    /// 
    /// Supports formats:
    /// - IP:PORT
    /// - IP:PORT:USER:PASS
    /// - USER:PASS@IP:PORT
    /// - scheme://IP:PORT
    /// - scheme://USER:PASS@IP:PORT
    pub fn parse_line(line: &str, default_type: ProxyType) -> Option<Proxy> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        // Try URL format first (e.g., http://ip:port or socks5://user:pass@ip:port)
        if let Some(proxy) = Self::parse_url_format(line) {
            return Some(proxy);
        }

        // Try user:pass@ip:port format
        if let Some(proxy) = Self::parse_auth_at_format(line, default_type.clone()) {
            return Some(proxy);
        }

        // Try ip:port:user:pass or ip:port format
        if let Some(proxy) = Self::parse_colon_format(line, default_type) {
            return Some(proxy);
        }

        None
    }

    /// Parse URL format proxy (e.g., http://ip:port or socks5://user:pass@ip:port)
    fn parse_url_format(line: &str) -> Option<Proxy> {
        let re = Regex::new(
            r"^(https?|socks[45])://(?:([^:]+):([^@]+)@)?([^:]+):(\d+)/?$"
        ).ok()?;

        let caps = re.captures(line)?;
        
        let proxy_type = match &caps[1] {
            "http" => ProxyType::Http,
            "https" => ProxyType::Https,
            "socks4" => ProxyType::Socks4,
            "socks5" => ProxyType::Socks5,
            _ => return None,
        };

        let host = caps[4].to_string();
        let port: u16 = caps[5].parse().ok()?;

        match (caps.get(2), caps.get(3)) {
            (Some(user), Some(pass)) => {
                Some(Proxy::with_auth(
                    host,
                    port,
                    proxy_type,
                    user.as_str().to_string(),
                    pass.as_str().to_string(),
                ))
            }
            _ => Some(Proxy::new(host, port, proxy_type)),
        }
    }

    /// Parse user:pass@ip:port format
    fn parse_auth_at_format(line: &str, default_type: ProxyType) -> Option<Proxy> {
        let re = Regex::new(r"^([^:]+):([^@]+)@([^:]+):(\d+)$").ok()?;
        let caps = re.captures(line)?;

        let username = caps[1].to_string();
        let password = caps[2].to_string();
        let host = caps[3].to_string();
        let port: u16 = caps[4].parse().ok()?;

        Some(Proxy::with_auth(host, port, default_type, username, password))
    }

    /// Parse ip:port or ip:port:user:pass format
    fn parse_colon_format(line: &str, default_type: ProxyType) -> Option<Proxy> {
        let parts: Vec<&str> = line.split(':').collect();
        
        match parts.len() {
            2 => {
                // IP:PORT format
                let host = parts[0].to_string();
                let port: u16 = parts[1].parse().ok()?;
                Some(Proxy::new(host, port, default_type))
            }
            4 => {
                // IP:PORT:USER:PASS format
                let host = parts[0].to_string();
                let port: u16 = parts[1].parse().ok()?;
                let username = parts[2].to_string();
                let password = parts[3].to_string();
                Some(Proxy::with_auth(host, port, default_type, username, password))
            }
            _ => None,
        }
    }

    /// Parse proxies from a string (multiple lines)
    pub fn parse_string(content: &str, default_type: ProxyType) -> Vec<Proxy> {
        content
            .lines()
            .filter_map(|line| Self::parse_line(line, default_type.clone()))
            .collect()
    }

    /// Parse proxies from a file
    pub fn parse_file<P: AsRef<Path>>(path: P, default_type: ProxyType) -> Result<Vec<Proxy>> {
        let content = fs::read_to_string(path)?;
        Ok(Self::parse_string(&content, default_type))
    }

    /// Save proxies to a file
    pub fn save_to_file<P: AsRef<Path>>(proxies: &[Proxy], path: P, full_format: bool) -> Result<()> {
        let content: String = proxies
            .iter()
            .map(|p| {
                if full_format {
                    p.to_full_string()
                } else {
                    p.to_simple_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_format() {
        let proxy = ProxyParser::parse_line("192.168.1.1:8080", ProxyType::Http).unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 8080);
        assert!(proxy.auth.is_none());
    }

    #[test]
    fn test_parse_with_auth_colon_format() {
        let proxy = ProxyParser::parse_line("192.168.1.1:8080:user:pass", ProxyType::Http).unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 8080);
        assert!(proxy.auth.is_some());
        let auth = proxy.auth.unwrap();
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn test_parse_auth_at_format() {
        let proxy = ProxyParser::parse_line("user:pass@192.168.1.1:8080", ProxyType::Http).unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 8080);
        assert!(proxy.auth.is_some());
        let auth = proxy.auth.unwrap();
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn test_parse_url_format_http() {
        let proxy = ProxyParser::parse_line("http://192.168.1.1:8080", ProxyType::Http).unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 8080);
        assert_eq!(proxy.proxy_type, ProxyType::Http);
    }

    #[test]
    fn test_parse_url_format_socks5() {
        let proxy = ProxyParser::parse_line("socks5://192.168.1.1:1080", ProxyType::Http).unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 1080);
        assert_eq!(proxy.proxy_type, ProxyType::Socks5);
    }

    #[test]
    fn test_parse_url_format_with_auth() {
        let proxy = ProxyParser::parse_line("socks5://user:pass@192.168.1.1:1080", ProxyType::Http).unwrap();
        assert_eq!(proxy.host, "192.168.1.1");
        assert_eq!(proxy.port, 1080);
        assert_eq!(proxy.proxy_type, ProxyType::Socks5);
        assert!(proxy.auth.is_some());
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(ProxyParser::parse_line("", ProxyType::Http).is_none());
    }

    #[test]
    fn test_parse_comment_line() {
        assert!(ProxyParser::parse_line("# This is a comment", ProxyType::Http).is_none());
    }

    #[test]
    fn test_parse_string() {
        let content = r#"
192.168.1.1:8080
192.168.1.2:8080:user:pass
# This is a comment
http://192.168.1.3:8080
"#;
        let proxies = ProxyParser::parse_string(content, ProxyType::Http);
        assert_eq!(proxies.len(), 3);
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(ProxyParser::parse_line("invalid", ProxyType::Http).is_none());
        assert!(ProxyParser::parse_line("192.168.1.1", ProxyType::Http).is_none());
        assert!(ProxyParser::parse_line("192.168.1.1:abc", ProxyType::Http).is_none());
    }
}
