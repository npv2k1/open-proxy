//! Geolocation module for detecting proxy IP location using MMDB

use crate::Result;
use maxminddb::{geoip2, Reader};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

/// Geographic location information for an IP address
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GeoLocation {
    /// ISO 3166-1 alpha-2 country code (e.g., "US", "CN")
    pub country_code: Option<String>,
    /// Country name in English
    pub country_name: Option<String>,
    /// City name in English
    pub city_name: Option<String>,
    /// Continent code (e.g., "NA", "EU", "AS")
    pub continent_code: Option<String>,
    /// Latitude coordinate
    pub latitude: Option<f64>,
    /// Longitude coordinate
    pub longitude: Option<f64>,
    /// Timezone (e.g., "America/New_York")
    pub timezone: Option<String>,
}

impl GeoLocation {
    /// Create a new GeoLocation with country information
    pub fn with_country(country_code: Option<String>, country_name: Option<String>) -> Self {
        Self {
            country_code,
            country_name,
            ..Default::default()
        }
    }

    /// Check if the location has any meaningful data
    pub fn is_empty(&self) -> bool {
        self.country_code.is_none()
            && self.country_name.is_none()
            && self.city_name.is_none()
            && self.continent_code.is_none()
    }

    /// Get a short display string for the location
    pub fn short_display(&self) -> String {
        match (&self.country_code, &self.city_name) {
            (Some(cc), Some(city)) => format!("{}, {}", city, cc),
            (Some(cc), None) => cc.clone(),
            (None, Some(city)) => city.clone(),
            (None, None) => String::from("Unknown"),
        }
    }
}

impl std::fmt::Display for GeoLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = [
            self.city_name.clone(),
            self.country_name.clone(),
            self.continent_code.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();

        if parts.is_empty() {
            write!(f, "Unknown Location")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

/// GeoLocator for looking up IP addresses in MMDB databases
pub struct GeoLocator {
    reader: Arc<Reader<Vec<u8>>>,
}

impl GeoLocator {
    /// Create a new GeoLocator from an MMDB file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = Reader::open_readfile(path)?;
        Ok(Self {
            reader: Arc::new(reader),
        })
    }

    /// Look up the geolocation for an IP address string
    pub fn lookup(&self, ip_str: &str) -> Result<GeoLocation> {
        let ip: IpAddr = ip_str.parse()?;
        self.lookup_ip(ip)
    }

    /// Look up the geolocation for an IpAddr
    pub fn lookup_ip(&self, ip: IpAddr) -> Result<GeoLocation> {
        let lookup_result = self.reader.lookup(ip)?;
        
        // Decode the City data from the lookup result
        let city: Option<geoip2::City> = lookup_result.decode()?;
        
        let Some(city) = city else {
            return Ok(GeoLocation::default());
        };

        // Access country information directly (not through Option)
        let country_code = city.country.iso_code.map(String::from);
        let country_name = city.country.names.english.map(String::from);
        
        // Access city name
        let city_name = city.city.names.english.map(String::from);
        
        // Access continent code
        let continent_code = city.continent.code.map(String::from);
        
        // Access location data
        let latitude = city.location.latitude;
        let longitude = city.location.longitude;
        let timezone = city.location.time_zone.map(String::from);

        Ok(GeoLocation {
            country_code,
            country_name,
            city_name,
            continent_code,
            latitude,
            longitude,
            timezone,
        })
    }
}

impl Clone for GeoLocator {
    fn clone(&self) -> Self {
        Self {
            reader: Arc::clone(&self.reader),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_location_default() {
        let loc = GeoLocation::default();
        assert!(loc.is_empty());
        assert_eq!(loc.short_display(), "Unknown");
    }

    #[test]
    fn test_geo_location_with_country() {
        let loc = GeoLocation::with_country(Some("US".to_string()), Some("United States".to_string()));
        assert!(!loc.is_empty());
        assert_eq!(loc.country_code, Some("US".to_string()));
        assert_eq!(loc.country_name, Some("United States".to_string()));
    }

    #[test]
    fn test_geo_location_short_display() {
        // Just country code
        let loc = GeoLocation::with_country(Some("US".to_string()), None);
        assert_eq!(loc.short_display(), "US");

        // Country code and city
        let mut loc = GeoLocation::with_country(Some("US".to_string()), None);
        loc.city_name = Some("New York".to_string());
        assert_eq!(loc.short_display(), "New York, US");

        // Just city
        let mut loc = GeoLocation::default();
        loc.city_name = Some("London".to_string());
        assert_eq!(loc.short_display(), "London");
    }

    #[test]
    fn test_geo_location_display() {
        let loc = GeoLocation {
            country_code: Some("US".to_string()),
            country_name: Some("United States".to_string()),
            city_name: Some("New York".to_string()),
            continent_code: Some("NA".to_string()),
            latitude: Some(40.7128),
            longitude: Some(-74.0060),
            timezone: Some("America/New_York".to_string()),
        };

        let display = format!("{}", loc);
        assert!(display.contains("New York"));
        assert!(display.contains("United States"));
        assert!(display.contains("NA"));
    }

    #[test]
    fn test_geo_location_empty_display() {
        let loc = GeoLocation::default();
        assert_eq!(format!("{}", loc), "Unknown Location");
    }
}
