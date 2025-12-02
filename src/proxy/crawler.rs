//! Proxy crawler module for fetching proxies from websites
//!
//! This module provides functionality for:
//! - Crawling proxy websites to extract proxy lists
//! - Parsing HTML/text content to find proxy entries
//! - Supporting multiple proxy source formats

use crate::proxy::models::{Proxy, ProxyType};
use crate::proxy::parser::ProxyParser;
use crate::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use std::time::Duration;

/// Default timeout for HTTP requests in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default user agent for HTTP requests
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";

/// Regex pattern to match IP:PORT patterns in text
static IP_PORT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}):(\d{1,5})\b")
        .expect("Invalid IP:PORT regex")
});

/// Result of crawling a single source
#[derive(Debug, Clone)]
pub struct CrawlResult {
    /// The source that was crawled
    pub source: String,
    /// Proxies extracted from the source
    pub proxies: Vec<Proxy>,
    /// Error message if crawling failed
    pub error: Option<String>,
}

impl CrawlResult {
    /// Create a successful crawl result
    pub fn success(source: String, proxies: Vec<Proxy>) -> Self {
        Self {
            source,
            proxies,
            error: None,
        }
    }

    /// Create a failed crawl result
    pub fn failure(source: String, error: String) -> Self {
        Self {
            source,
            proxies: Vec::new(),
            error: Some(error),
        }
    }

    /// Check if the crawl was successful
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// Configuration for proxy crawler
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    /// Timeout for HTTP requests
    pub timeout: Duration,
    /// User agent for HTTP requests
    pub user_agent: String,
    /// Default proxy type for parsed proxies
    pub default_proxy_type: ProxyType,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            default_proxy_type: ProxyType::Http,
        }
    }
}

impl CrawlerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = user_agent;
        self
    }

    pub fn with_proxy_type(mut self, proxy_type: ProxyType) -> Self {
        self.default_proxy_type = proxy_type;
        self
    }
}

/// Proxy source representing a website that provides proxy lists
#[derive(Debug, Clone)]
pub struct ProxySource {
    /// Name of the proxy source
    pub name: String,
    /// URL to fetch proxies from
    pub url: String,
    /// Proxy type for this source
    pub proxy_type: ProxyType,
}

impl ProxySource {
    pub fn new(name: &str, url: &str, proxy_type: ProxyType) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
            proxy_type,
        }
    }
}

/// Proxy crawler for fetching proxies from websites
pub struct ProxyCrawler {
    config: CrawlerConfig,
    client: Client,
}

impl ProxyCrawler {
    /// Create a new proxy crawler with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(CrawlerConfig::default())
    }

    /// Create a new proxy crawler with custom configuration
    pub fn with_config(config: CrawlerConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()?;

        Ok(Self { config, client })
    }

    /// Get the default proxy type from configuration
    pub fn default_proxy_type(&self) -> &ProxyType {
        &self.config.default_proxy_type
    }

    /// Fetch and parse proxies from a single URL using default proxy type
    pub async fn crawl_url_default(&self, url: &str) -> Result<Vec<Proxy>> {
        self.crawl_url(url, self.config.default_proxy_type.clone())
            .await
    }

    /// Fetch and parse proxies from a single URL
    pub async fn crawl_url(&self, url: &str, proxy_type: ProxyType) -> Result<Vec<Proxy>> {
        let response = self.client.get(url).send().await?;
        let content = response.text().await?;
        Ok(self.parse_proxies_from_text(&content, proxy_type))
    }

    /// Fetch and parse proxies from multiple URLs, returning results for each
    pub async fn crawl_urls_with_results(&self, urls: &[(&str, ProxyType)]) -> Vec<CrawlResult> {
        let mut results = Vec::new();

        for (url, proxy_type) in urls {
            let result = match self.crawl_url(url, proxy_type.clone()).await {
                Ok(proxies) => CrawlResult::success(url.to_string(), proxies),
                Err(e) => CrawlResult::failure(url.to_string(), e.to_string()),
            };
            results.push(result);
        }

        results
    }

    /// Fetch and parse proxies from a ProxySource
    pub async fn crawl_source(&self, source: &ProxySource) -> Result<Vec<Proxy>> {
        self.crawl_url(&source.url, source.proxy_type.clone()).await
    }

    /// Fetch and parse proxies from multiple ProxySources, returning results for each
    pub async fn crawl_sources_with_results(&self, sources: &[ProxySource]) -> Vec<CrawlResult> {
        let mut results = Vec::new();

        for source in sources {
            let result = match self.crawl_source(source).await {
                Ok(proxies) => CrawlResult::success(source.name.clone(), proxies),
                Err(e) => CrawlResult::failure(source.name.clone(), e.to_string()),
            };
            results.push(result);
        }

        results
    }

    /// Parse proxies from raw text content
    ///
    /// This method tries multiple parsing strategies:
    /// 1. Line-by-line parsing using ProxyParser
    /// 2. Regex-based IP:PORT extraction
    pub fn parse_proxies_from_text(&self, content: &str, proxy_type: ProxyType) -> Vec<Proxy> {
        let mut proxies = Vec::new();

        // First, try line-by-line parsing using the existing parser
        for line in content.lines() {
            if let Some(proxy) = ProxyParser::parse_line(line, proxy_type.clone()) {
                proxies.push(proxy);
            }
        }

        // If no proxies found, try regex-based extraction
        if proxies.is_empty() {
            proxies = self.extract_proxies_with_regex(content, proxy_type);
        }

        // Remove duplicates based on host:port
        proxies.sort_by(|a, b| {
            let key_a = format!("{}:{}", a.host, a.port);
            let key_b = format!("{}:{}", b.host, b.port);
            key_a.cmp(&key_b)
        });
        proxies.dedup_by(|a, b| a.host == b.host && a.port == b.port);

        proxies
    }

    /// Extract proxies using regex pattern matching
    fn extract_proxies_with_regex(&self, content: &str, proxy_type: ProxyType) -> Vec<Proxy> {
        IP_PORT_REGEX
            .captures_iter(content)
            .filter_map(|cap| {
                let host = cap.get(1)?.as_str().to_string();
                let port: u16 = cap.get(2)?.as_str().parse().ok()?;

                // Validate IP address parts
                let parts: Vec<&str> = host.split('.').collect();
                if parts.len() != 4 {
                    return None;
                }
                for part in parts {
                    let num: u32 = part.parse().ok()?;
                    if num > 255 {
                        return None;
                    }
                }

                // Validate port
                if port == 0 {
                    return None;
                }

                Some(Proxy::new(host, port, proxy_type.clone()))
            })
            .collect()
    }

    /// Get a list of common free proxy sources
    pub fn get_common_sources() -> Vec<ProxySource> {
        vec![
            ProxySource::new(
                "free-proxy-list.net",
                "https://free-proxy-list.net/",
                ProxyType::Http,
            ),
            ProxySource::new(
                "sslproxies",
                "https://www.sslproxies.org/",
                ProxyType::Https,
            ),
            ProxySource::new("us-proxy.org", "https://www.us-proxy.org/", ProxyType::Http),
            ProxySource::new(
                "socks-proxy.net",
                "https://www.socks-proxy.net/",
                ProxyType::Socks4,
            ),
        ]
    }
}

impl Default for ProxyCrawler {
    fn default() -> Self {
        Self::new().expect("Failed to create default ProxyCrawler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crawler_config_default() {
        let config = CrawlerConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert_eq!(config.user_agent, DEFAULT_USER_AGENT);
        assert_eq!(config.default_proxy_type, ProxyType::Http);
    }

    #[test]
    fn test_crawler_config_builder() {
        let config = CrawlerConfig::new()
            .with_timeout(Duration::from_secs(60))
            .with_user_agent("Custom Agent".to_string())
            .with_proxy_type(ProxyType::Socks5);

        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.user_agent, "Custom Agent");
        assert_eq!(config.default_proxy_type, ProxyType::Socks5);
    }

    #[test]
    fn test_proxy_source_creation() {
        let source = ProxySource::new(
            "test-source",
            "https://example.com/proxies.txt",
            ProxyType::Http,
        );
        assert_eq!(source.name, "test-source");
        assert_eq!(source.url, "https://example.com/proxies.txt");
        assert_eq!(source.proxy_type, ProxyType::Http);
    }

    #[test]
    fn test_crawl_result_success() {
        let proxies = vec![
            Proxy::new("192.168.1.1".to_string(), 8080, ProxyType::Http),
            Proxy::new("192.168.1.2".to_string(), 3128, ProxyType::Http),
        ];
        let result = CrawlResult::success("test-source".to_string(), proxies);
        assert!(result.is_success());
        assert_eq!(result.source, "test-source");
        assert_eq!(result.proxies.len(), 2);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_crawl_result_failure() {
        let result =
            CrawlResult::failure("test-source".to_string(), "Connection failed".to_string());
        assert!(!result.is_success());
        assert_eq!(result.source, "test-source");
        assert!(result.proxies.is_empty());
        assert_eq!(result.error, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_parse_proxies_from_text_simple() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = r#"
192.168.1.1:8080
192.168.1.2:3128
10.0.0.1:1080
"#;
        let proxies = crawler.parse_proxies_from_text(content, ProxyType::Http);
        assert_eq!(proxies.len(), 3);
    }

    #[test]
    fn test_parse_proxies_from_text_with_comments() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = r#"
# HTTP Proxies
192.168.1.1:8080
# Another comment
192.168.1.2:3128
"#;
        let proxies = crawler.parse_proxies_from_text(content, ProxyType::Http);
        assert_eq!(proxies.len(), 2);
    }

    #[test]
    fn test_parse_proxies_from_html_like_content() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = r#"
<html>
<body>
<table>
<tr><td>192.168.1.1</td><td>8080</td></tr>
</table>
Some text with 10.0.0.1:3128 embedded
</body>
</html>
"#;
        let proxies = crawler.parse_proxies_from_text(content, ProxyType::Http);
        // Should extract 10.0.0.1:3128 via regex
        assert!(!proxies.is_empty());
        assert!(proxies
            .iter()
            .any(|p| p.host == "10.0.0.1" && p.port == 3128));
    }

    #[test]
    fn test_parse_proxies_deduplication() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = r#"
192.168.1.1:8080
192.168.1.1:8080
192.168.1.2:3128
192.168.1.1:8080
"#;
        let proxies = crawler.parse_proxies_from_text(content, ProxyType::Http);
        // Should deduplicate to 2 unique proxies
        assert_eq!(proxies.len(), 2);
    }

    #[test]
    fn test_extract_proxies_with_regex() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = "Here is a proxy: 192.168.1.1:8080 and another one 10.0.0.1:3128.";
        let proxies = crawler.extract_proxies_with_regex(content, ProxyType::Http);
        assert_eq!(proxies.len(), 2);
    }

    #[test]
    fn test_extract_proxies_invalid_ip() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = "Invalid IP: 999.999.999.999:8080";
        let proxies = crawler.extract_proxies_with_regex(content, ProxyType::Http);
        assert!(proxies.is_empty());
    }

    #[test]
    fn test_extract_proxies_invalid_port() {
        let crawler = ProxyCrawler::new().unwrap();
        let content = "Zero port: 192.168.1.1:0";
        let proxies = crawler.extract_proxies_with_regex(content, ProxyType::Http);
        assert!(proxies.is_empty());
    }

    #[test]
    fn test_get_common_sources() {
        let sources = ProxyCrawler::get_common_sources();
        assert!(!sources.is_empty());
        for source in &sources {
            assert!(!source.name.is_empty());
            assert!(!source.url.is_empty());
            assert!(source.url.starts_with("http"));
        }
    }
}
