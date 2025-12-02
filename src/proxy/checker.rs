//! Proxy checker module for checking proxy validity

use crate::proxy::geo::GeoLocator;
use crate::proxy::models::{Proxy, ProxyCheckResult, ProxyType};
use crate::Result;
use futures::stream::{self, StreamExt};
use reqwest::{Client, Proxy as ReqwestProxy};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Default timeout for proxy checks in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Default number of concurrent checks
const DEFAULT_CONCURRENCY: usize = 10;

/// Default URL to test proxies against
const DEFAULT_TEST_URL: &str = "http://httpbin.org/ip";

/// Configuration for proxy checker
#[derive(Debug, Clone)]
pub struct CheckerConfig {
    /// Timeout for each proxy check
    pub timeout: Duration,
    /// Number of concurrent checks
    pub concurrency: usize,
    /// URL to test proxies against
    pub test_url: String,
    /// Path to MMDB file for geolocation (optional)
    pub mmdb_path: Option<String>,
}

impl Default for CheckerConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            concurrency: DEFAULT_CONCURRENCY,
            test_url: DEFAULT_TEST_URL.to_string(),
            mmdb_path: None,
        }
    }
}

impl CheckerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn with_test_url(mut self, url: String) -> Self {
        self.test_url = url;
        self
    }

    pub fn with_mmdb_path(mut self, path: String) -> Self {
        self.mmdb_path = Some(path);
        self
    }
}

/// Proxy checker for validating proxies
pub struct ProxyChecker {
    config: CheckerConfig,
    geo_locator: Option<GeoLocator>,
}

impl ProxyChecker {
    /// Create a new proxy checker with default configuration
    pub fn new() -> Self {
        Self {
            config: CheckerConfig::default(),
            geo_locator: None,
        }
    }

    /// Create a new proxy checker with custom configuration
    pub fn with_config(config: CheckerConfig) -> Self {
        let geo_locator = config.mmdb_path.as_ref().and_then(|path| {
            GeoLocator::from_path(path).ok()
        });
        
        Self { config, geo_locator }
    }

    /// Check a single proxy
    pub async fn check_proxy(&self, proxy: &Proxy) -> ProxyCheckResult {
        let start = Instant::now();
        
        match self.create_client(proxy) {
            Ok(client) => {
                match tokio::time::timeout(
                    self.config.timeout,
                    client.get(&self.config.test_url).send(),
                )
                .await
                {
                    Ok(Ok(response)) => {
                        if response.status().is_success() {
                            let elapsed = start.elapsed().as_millis() as u64;
                            let mut result = ProxyCheckResult::working(proxy.clone(), elapsed);
                            
                            // Attempt to get geolocation if configured
                            if let Some(ref geo) = self.geo_locator {
                                if let Ok(location) = geo.lookup(&proxy.host) {
                                    result = result.with_geo_location(location);
                                }
                            }
                            
                            result
                        } else {
                            ProxyCheckResult::failed(
                                proxy.clone(),
                                format!("HTTP status: {}", response.status()),
                            )
                        }
                    }
                    Ok(Err(e)) => {
                        ProxyCheckResult::failed(proxy.clone(), e.to_string())
                    }
                    Err(_) => ProxyCheckResult::timeout(proxy.clone()),
                }
            }
            Err(e) => ProxyCheckResult::failed(proxy.clone(), e.to_string()),
        }
    }

    /// Check multiple proxies concurrently
    pub async fn check_proxies(&self, proxies: Vec<Proxy>) -> Vec<ProxyCheckResult> {
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        let results = stream::iter(proxies)
            .map(|proxy| {
                let sem = Arc::clone(&semaphore);
                let checker = self.clone();
                async move {
                    // Semaphore acquire only fails if the semaphore is closed,
                    // which won't happen here since we own the Arc and keep it alive
                    // for the duration of the check operation.
                    let _permit = sem
                        .acquire()
                        .await
                        .expect("Semaphore closed unexpectedly");
                    checker.check_proxy(&proxy).await
                }
            })
            .buffer_unordered(self.config.concurrency)
            .collect::<Vec<_>>()
            .await;

        results
    }

    /// Check proxies and separate into good and bad results
    pub async fn check_and_separate(
        &self,
        proxies: Vec<Proxy>,
    ) -> (Vec<ProxyCheckResult>, Vec<ProxyCheckResult>) {
        let results = self.check_proxies(proxies).await;
        
        let (good, bad): (Vec<_>, Vec<_>) = results.into_iter().partition(|r| r.is_working());
        
        (good, bad)
    }

    /// Create a reqwest client with the proxy
    fn create_client(&self, proxy: &Proxy) -> Result<Client> {
        let proxy_url = proxy.url();
        
        let reqwest_proxy = match proxy.proxy_type {
            ProxyType::Http | ProxyType::Https => {
                ReqwestProxy::http(&proxy_url)?
            }
            ProxyType::Socks4 | ProxyType::Socks5 => {
                ReqwestProxy::all(&proxy_url)?
            }
        };

        let client = Client::builder()
            .proxy(reqwest_proxy)
            .timeout(self.config.timeout)
            .build()?;

        Ok(client)
    }
}

impl Clone for ProxyChecker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            geo_locator: self.geo_locator.clone(),
        }
    }
}

impl Default for ProxyChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checker_config_default() {
        let config = CheckerConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        assert_eq!(config.concurrency, DEFAULT_CONCURRENCY);
        assert_eq!(config.test_url, DEFAULT_TEST_URL);
    }

    #[test]
    fn test_checker_config_builder() {
        let config = CheckerConfig::new()
            .with_timeout(Duration::from_secs(30))
            .with_concurrency(20)
            .with_test_url("http://example.com".to_string());

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.concurrency, 20);
        assert_eq!(config.test_url, "http://example.com");
    }

    #[test]
    fn test_proxy_checker_creation() {
        let checker = ProxyChecker::new();
        assert_eq!(checker.config.concurrency, DEFAULT_CONCURRENCY);
    }

    #[test]
    fn test_proxy_checker_with_config() {
        let config = CheckerConfig::new().with_concurrency(50);
        let checker = ProxyChecker::with_config(config);
        assert_eq!(checker.config.concurrency, 50);
    }
}
