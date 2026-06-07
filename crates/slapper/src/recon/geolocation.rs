use crate::error::{Result, SlapperError};
use crate::types::SensitiveString;
use crate::utils::create_http_client_with_options;
use maxminddb::geoip2;
use maxminddb::Reader;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::LazyLock;
use urlencoding;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoLocation {
    pub ip: String,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub timezone: Option<String>,
    pub coordinates: Option<String>,
    pub asn: Option<String>,
}

static LOCAL_IP_DATA: LazyLock<FxHashMap<String, (String, String, String)>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    m.insert(
        "10.0.0.0/8".to_string(),
        (
            "Private Network".to_string(),
            "XX".to_string(),
            "Local".to_string(),
        ),
    );
    m.insert(
        "172.16.0.0/12".to_string(),
        (
            "Private Network".to_string(),
            "XX".to_string(),
            "Local".to_string(),
        ),
    );
    m.insert(
        "192.168.0.0/16".to_string(),
        (
            "Private Network".to_string(),
            "XX".to_string(),
            "Local".to_string(),
        ),
    );
    m.insert(
        "127.0.0.0/8".to_string(),
        (
            "Loopback".to_string(),
            "XX".to_string(),
            "Localhost".to_string(),
        ),
    );
    m.insert(
        "224.0.0.0/4".to_string(),
        (
            "Multicast".to_string(),
            "XX".to_string(),
            "Multicast".to_string(),
        ),
    );
    m
});

#[derive(Debug, Clone, Default)]
pub struct MaxMindSettings {
    pub account_id: Option<u32>,
    pub license_key: Option<SensitiveString>,
    pub edition_ids: Vec<String>,
    pub data_dir: PathBuf,
    pub auto_update: bool,
}

pub struct GeoLocator {
    client: reqwest::Client,
    use_online: bool,
    ipapi_key: Option<SensitiveString>,
    maxmind_settings: Option<MaxMindSettings>,
    maxmind_db_path: Option<PathBuf>,
    maxmind_reader: Option<Reader<Vec<u8>>>,
}

impl GeoLocator {
    pub fn new() -> Result<Self> {
        let client = create_http_client_with_options(10, |builder| {
            builder.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        })?;

        Ok(Self {
            client,
            use_online: true,
            ipapi_key: None,
            maxmind_settings: None,
            maxmind_db_path: None,
            maxmind_reader: None,
        })
    }

    pub fn with_online_disabled() -> Result<Self> {
        let client = create_http_client_with_options(10, |builder| builder)?;
        Ok(Self {
            client,
            use_online: false,
            ipapi_key: None,
            maxmind_settings: None,
            maxmind_db_path: None,
            maxmind_reader: None,
        })
    }

    pub fn set_online(&mut self, enabled: bool) {
        self.use_online = enabled;
    }

    pub fn set_ipapi_key(&mut self, key: Option<SensitiveString>) {
        self.ipapi_key = key;
    }

    pub fn set_maxmind_settings(&mut self, settings: MaxMindSettings) {
        self.maxmind_settings = Some(settings);
    }

    pub async fn init_geoip(&mut self) -> Result<()> {
        let settings = &self.maxmind_settings;

        let data_dir = settings
            .as_ref()
            .map(|s| s.data_dir.clone())
            .unwrap_or_else(|| PathBuf::from(".slapper/geoip"));

        tokio::fs::create_dir_all(&data_dir).await?;

        let ip66_path = data_dir.join("ip66.mmdb");
        let maxmind_path = data_dir.join("GeoLite2-City.mmdb");

        if !ip66_path.exists() && !maxmind_path.exists() {
            if let Some(ref s) = settings {
                if s.account_id.is_some() && s.license_key.is_some() {
                    self.download_maxmind_db(settings.as_ref().unwrap())
                        .await?;
                } else {
                    tracing::info!("Downloading free IP66 database...");
                    self.download_ip66_db(&ip66_path).await?;
                }
            } else {
                tracing::info!("Downloading free IP66 database...");
                self.download_ip66_db(&ip66_path).await?;
            }
        }

        if ip66_path.exists() {
            self.maxmind_db_path = Some(ip66_path.clone());
            self.init_maxmind_reader(&ip66_path)?;
            tracing::info!("IP66 database configured at {:?}", ip66_path);
        } else if maxmind_path.exists() {
            self.maxmind_db_path = Some(maxmind_path.clone());
            self.init_maxmind_reader(&maxmind_path)?;
            tracing::info!("MaxMind database configured at {:?}", maxmind_path);
        }

        Ok(())
    }

    fn init_maxmind_reader(&mut self, db_path: &PathBuf) -> Result<()> {
        let reader = Reader::<Vec<u8>>::open_readfile(db_path)?;
        self.maxmind_reader = Some(reader);
        Ok(())
    }

    pub async fn init_maxmind(&mut self) -> Result<()> {
        self.init_geoip().await
    }

    async fn download_ip66_db(&self, db_path: &PathBuf) -> Result<()> {
        let url = "https://downloads.ip66.dev/db/ip66.mmdb";

        tracing::info!("Downloading IP66 GeoIP database from {}", url);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(SlapperError::Network(format!(
                "Failed to download IP66 DB: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        tokio::fs::write(db_path, &bytes).await?;

        tracing::info!("IP66 database downloaded to {:?}", db_path);

        Ok(())
    }

    async fn download_maxmind_db(&self, settings: &MaxMindSettings) -> Result<()> {
        let account_id = settings
            .account_id
            .ok_or_else(|| SlapperError::Config("MaxMind account_id required".to_string()))?;
        let license_key = settings
            .license_key
            .as_ref()
            .ok_or_else(|| SlapperError::Config("MaxMind license_key required".to_string()))?;

        let data_dir = &settings.data_dir;
        tokio::fs::create_dir_all(data_dir).await?;

        let url = format!(
            "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key={}&suffix=mmdb",
            license_key.expose_secret()
        );

        tracing::info!("Downloading MaxMind GeoLite2-City database...");

        let response = self
            .client
            .get(&url)
            .basic_auth(account_id.to_string(), Some(license_key.expose_secret()))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SlapperError::Network(format!(
                "Failed to download MaxMind DB: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        let db_path = data_dir.join("GeoLite2-City.mmdb");

        tokio::fs::write(&db_path, &bytes).await?;

        tracing::info!("MaxMind database downloaded to {:?}", db_path);

        Ok(())
    }

    pub async fn lookup(&self, ip: &str) -> Result<GeoLocation> {
        if let Some(local) = self.lookup_local(ip) {
            return Ok(local);
        }

        if !self.use_online {
            return self.unknown_geo(ip);
        }

        if let Some(geoip_result) = self.lookup_maxmind(ip).await {
            return Ok(geoip_result);
        }

        if let Ok(loc) = self.lookup_vuiz_net(ip).await {
            return Ok(loc);
        }

        if let Ok(loc) = self.lookup_ipapi(ip).await {
            return Ok(loc);
        }

        if let Ok(loc) = self.lookup_ip_api_com(ip).await {
            return Ok(loc);
        }

        if let Ok(loc) = self.lookup_ipwhois_io(ip).await {
            return Ok(loc);
        }

        if let Ok(loc) = self.lookup_ip2c(ip).await {
            return Ok(loc);
        }

        self.unknown_geo(ip)
    }

    fn unknown_geo(&self, ip: &str) -> Result<GeoLocation> {
        Ok(GeoLocation {
            ip: ip.to_string(),
            country: Some("Unknown".to_string()),
            country_code: Some("XX".to_string()),
            region: None,
            city: None,
            isp: None,
            org: None,
            timezone: None,
            coordinates: None,
            asn: None,
        })
    }

    fn lookup_local(&self, ip: &str) -> Option<GeoLocation> {
        let addr: Ipv4Addr = ip.parse().ok()?;
        let addr_u32 = u32::from(addr);

        for (cidr, (country, code, region)) in LOCAL_IP_DATA.iter() {
            if cidr.contains('/') {
                let parts: Vec<&str> = cidr.split('/').collect();
                if parts.len() != 2 {
                    continue;
                }
                let base: Ipv4Addr = parts[0].parse().ok()?;
                let prefix: u8 = parts[1].parse().ok()?;
                let mask = u32::MAX << (32 - prefix);
                if (addr_u32 & mask) == (u32::from(base) & mask) {
                    return Some(GeoLocation {
                        ip: ip.to_string(),
                        country: Some(country.clone()),
                        country_code: Some(code.clone()),
                        region: Some(region.clone()),
                        city: None,
                        isp: None,
                        org: None,
                        timezone: None,
                        coordinates: None,
                        asn: None,
                    });
                }
            }
        }

        None
    }

    async fn lookup_maxmind(&self, ip: &str) -> Option<GeoLocation> {
        let reader = self.maxmind_reader.as_ref()?;

        let ip_addr: IpAddr = ip.parse().ok()?;

        let result = reader.lookup(ip_addr).ok()?;

        let city = result.decode::<geoip2::City>().ok().flatten()?;

        let country_code = city.country.iso_code.map(String::from);
        let country = city.country.names.english.map(|s| s.to_string());
        let region = city
            .subdivisions
            .first()
            .and_then(|s| s.names.english)
            .map(String::from);
        let city_name = city.city.names.english.map(|s| s.to_string());

        let isp = city.traits.is_anycast.map(|a| {
            if a {
                "Anycast".to_string()
            } else {
                "No".to_string()
            }
        });

        let coordinates = match (city.location.latitude, city.location.longitude) {
            (Some(lat), Some(lon)) => Some(format!("{}, {}", lat, lon)),
            _ => None,
        };

        let timezone = city.location.time_zone.map(String::from);

        Some(GeoLocation {
            ip: ip.to_string(),
            country,
            country_code,
            region,
            city: city_name,
            isp: None,
            org: isp,
            timezone,
            coordinates,
            asn: None,
        })
    }

    async fn lookup_vuiz_net(&self, ip: &str) -> Result<GeoLocation> {
        let url = format!("https://geoip.vuiz.net/geoip?ip={}", ip);

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct VuizResponse {
            ip: String,
            #[serde(rename = "country")]
            country: Option<String>,
            #[serde(rename = "countryCode")]
            country_code: Option<String>,
            #[serde(rename = "region")]
            region: Option<String>,
            #[serde(rename = "city")]
            city: Option<String>,
            #[serde(rename = "isp")]
            isp: Option<String>,
            #[serde(rename = "org")]
            org: Option<String>,
            #[serde(rename = "timezone")]
            timezone: Option<String>,
            #[serde(rename = "lat")]
            lat: Option<f64>,
            #[serde(rename = "lon")]
            lon: Option<f64>,
        }

        let api_resp: VuizResponse = response.json().await?;

        let coordinates = match (api_resp.lat, api_resp.lon) {
            (Some(lat), Some(lon)) => Some(format!("{}, {}", lat, lon)),
            _ => None,
        };

        Ok(GeoLocation {
            ip: api_resp.ip,
            country: api_resp.country,
            country_code: api_resp.country_code,
            region: api_resp.region,
            city: api_resp.city,
            isp: api_resp.isp,
            org: api_resp.org,
            timezone: api_resp.timezone,
            coordinates,
            asn: None,
        })
    }

    async fn lookup_ipapi(&self, ip: &str) -> Result<GeoLocation> {
        let url = if let Some(ref key) = self.ipapi_key {
            format!(
                "https://ipapi.co/{}/json/?key={}",
                ip,
                urlencoding::encode(key.expose_secret())
            )
        } else {
            format!("https://ipapi.co/{}/json/", ip)
        };

        let response = self.client.get(&url).send().await?;

        if response.status() == crate::constants::STATUS_RATE_LIMITED || response.status() == crate::constants::STATUS_FORBIDDEN {
            return Err(SlapperError::RateLimited(
                "Rate limited or forbidden".to_string(),
            ));
        }

        #[derive(Deserialize)]
        struct IpApiResponse {
            #[serde(rename = "ip")]
            ip: Option<String>,
            #[serde(rename = "country")]
            country: Option<String>,
            #[serde(rename = "country_code")]
            country_code: Option<String>,
            #[serde(rename = "region")]
            region: Option<String>,
            #[serde(rename = "city")]
            city: Option<String>,
            #[serde(rename = "org")]
            org: Option<String>,
            #[serde(rename = "timezone")]
            timezone: Option<String>,
            #[serde(rename = "latitude")]
            lat: Option<f64>,
            #[serde(rename = "longitude")]
            lon: Option<f64>,
            #[serde(rename = "asn")]
            asn: Option<String>,
            #[serde(rename = "isp")]
            isp: Option<String>,
        }

        let api_resp: IpApiResponse = response.json().await?;

        let coordinates = match (api_resp.lat, api_resp.lon) {
            (Some(lat), Some(lon)) => Some(format!("{}, {}", lat, lon)),
            _ => None,
        };

        Ok(GeoLocation {
            ip: api_resp.ip.unwrap_or_else(|| ip.to_string()),
            country: api_resp.country,
            country_code: api_resp.country_code,
            region: api_resp.region,
            city: api_resp.city,
            isp: api_resp.isp,
            org: api_resp.org,
            timezone: api_resp.timezone,
            coordinates,
            asn: api_resp.asn,
        })
    }

    async fn lookup_ip_api_com(&self, ip: &str) -> Result<GeoLocation> {
        let url = format!("http://ip-api.com/json/{}", ip);

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct FallbackResponse {
            #[serde(rename = "query")]
            ip: String,
            #[serde(rename = "country")]
            country: Option<String>,
            #[serde(rename = "countryCode")]
            country_code: Option<String>,
            #[serde(rename = "regionName")]
            region: Option<String>,
            #[serde(rename = "city")]
            city: Option<String>,
            #[serde(rename = "isp")]
            isp: Option<String>,
            #[serde(rename = "org")]
            org: Option<String>,
            #[serde(rename = "timezone")]
            timezone: Option<String>,
            #[serde(rename = "lat")]
            lat: Option<f64>,
            #[serde(rename = "lon")]
            lon: Option<f64>,
            #[serde(rename = "as")]
            asn: Option<String>,
        }

        let api_resp: FallbackResponse = response.json().await?;

        let coordinates = match (api_resp.lat, api_resp.lon) {
            (Some(lat), Some(lon)) => Some(format!("{}, {}", lat, lon)),
            _ => None,
        };

        Ok(GeoLocation {
            ip: api_resp.ip,
            country: api_resp.country,
            country_code: api_resp.country_code,
            region: api_resp.region,
            city: api_resp.city,
            isp: api_resp.isp,
            org: api_resp.org,
            timezone: api_resp.timezone,
            coordinates,
            asn: api_resp.asn,
        })
    }

    async fn lookup_ipwhois_io(&self, ip: &str) -> Result<GeoLocation> {
        let url = format!("https://ipwho.is/{}", ip);

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct IpWhoisResponse {
            ip: String,
            #[serde(rename = "country")]
            country: Option<String>,
            #[serde(rename = "country_code")]
            country_code: Option<String>,
            #[serde(rename = "region")]
            region: Option<String>,
            #[serde(rename = "city")]
            city: Option<String>,
            #[serde(rename = "isp")]
            isp: Option<String>,
            #[serde(rename = "org")]
            org: Option<String>,
            #[serde(rename = "timezone")]
            timezone: Option<String>,
            #[serde(rename = "latitude")]
            lat: Option<f64>,
            #[serde(rename = "longitude")]
            lon: Option<f64>,
            #[serde(rename = "asn")]
            asn: Option<String>,
        }

        let api_resp: IpWhoisResponse = response.json().await?;

        let coordinates = match (api_resp.lat, api_resp.lon) {
            (Some(lat), Some(lon)) => Some(format!("{}, {}", lat, lon)),
            _ => None,
        };

        Ok(GeoLocation {
            ip: api_resp.ip,
            country: api_resp.country,
            country_code: api_resp.country_code,
            region: api_resp.region,
            city: api_resp.city,
            isp: api_resp.isp,
            org: api_resp.org,
            timezone: api_resp.timezone,
            coordinates,
            asn: api_resp.asn,
        })
    }

    async fn lookup_ip2c(&self, ip: &str) -> Result<GeoLocation> {
        let url = format!("https://ip2c.org/{}", ip);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;

        let parts: Vec<&str> = text.split(';').collect();

        if parts.first() == Some(&"1") && parts.len() >= 4 {
            return Ok(GeoLocation {
                ip: ip.to_string(),
                country: parts.get(3).map(|s| s.to_string()),
                country_code: parts.get(1).map(|s| s.to_string()),
                region: None,
                city: None,
                isp: None,
                org: None,
                timezone: None,
                coordinates: None,
                asn: None,
            });
        }

        Err(SlapperError::Network(format!(
            "ip2c lookup failed: {}",
            text
        )))
    }
}

pub async fn geolocation_lookup(ip: &str) -> Result<GeoLocation> {
    let locator = GeoLocator::new()?;
    locator.lookup(ip).await
}

pub async fn geolocation_lookup_with_config(
    ip: &str,
    ipapi_key: Option<&SensitiveString>,
    maxmind_settings: Option<MaxMindSettings>,
) -> Result<GeoLocation> {
    let mut locator = GeoLocator::new()?;

    if let Some(key) = ipapi_key {
        locator.set_ipapi_key(Some(key.clone()));
    }

    if let Some(settings) = maxmind_settings.clone() {
        locator.set_maxmind_settings(settings);
        if let Err(e) = locator.init_geoip().await {
            tracing::warn!("MaxMind GeoIP initialization failed: {}", e);
        }
    }

    locator.lookup(ip).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_ip_detection() {
        let locator = GeoLocator::new().unwrap();

        let result = locator.lookup_local("127.0.0.1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().country_code, Some("XX".to_string()));

        let result = locator.lookup_local("192.168.1.1");
        assert!(result.is_some());

        let result = locator.lookup_local("10.0.0.1");
        assert!(result.is_some());

        let result = locator.lookup_local("8.8.8.8");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_offline_locator() {
        let locator = GeoLocator::with_online_disabled().unwrap();

        let result = locator.lookup("8.8.8.8").await;
        assert!(result.is_ok());
    }
}
