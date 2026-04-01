use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthConfig {
    pub user_agents: Vec<String>,
    pub jitter_min_ms: u64,
    pub jitter_max_ms: u64,
    pub rotate_headers: bool,
    pub browser_fingerprint: BrowserFingerprint,
    pub tls_fingerprint: TlsFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFingerprint {
    pub screen_resolution: (u32, u32),
    pub timezone: String,
    pub language: String,
    pub platform: String,
    pub hardware_concurrency: u32,
    pub device_memory: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsFingerprint {
    pub ja3_hash: String,
    pub supported_versions: Vec<String>,
    pub cipher_suites: Vec<u16>,
    pub extensions: Vec<u16>,
    pub curves: Vec<String>,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            user_agents: default_user_agents(),
            jitter_min_ms: 0,
            jitter_max_ms: 0,
            rotate_headers: false,
            browser_fingerprint: BrowserFingerprint::random(),
            tls_fingerprint: TlsFingerprint::chrome(),
        }
    }
}

impl Default for BrowserFingerprint {
    fn default() -> Self {
        Self::random()
    }
}

impl BrowserFingerprint {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();

        let resolutions = [
            (1920, 1080),
            (1366, 768),
            (1536, 864),
            (1440, 900),
            (1280, 720),
        ];
        let resolution = resolutions[rng.gen_range(0..resolutions.len())];

        let timezones = [
            "America/New_York",
            "America/Los_Angeles",
            "America/Chicago",
            "Europe/London",
            "Europe/Paris",
            "Asia/Tokyo",
            "UTC",
        ];

        let languages = [
            "en-US,en;q=0.9",
            "en-GB,en;q=0.9",
            "en-US,en;q=0.5",
            "en,en-US;q=0.9",
        ];

        Self {
            screen_resolution: resolution,
            timezone: timezones[rng.gen_range(0..timezones.len())].to_string(),
            language: languages[rng.gen_range(0..languages.len())].to_string(),
            platform: "Win32".to_string(),
            hardware_concurrency: rng.gen_range(4..16),
            device_memory: Some(rng.gen_range(4..32)),
        }
    }

    pub fn chrome() -> Self {
        Self {
            screen_resolution: (1920, 1080),
            timezone: "America/New_York".to_string(),
            language: "en-US,en;q=0.9".to_string(),
            platform: "Win32".to_string(),
            hardware_concurrency: 8,
            device_memory: Some(8),
        }
    }

    pub fn firefox() -> Self {
        Self {
            screen_resolution: (1920, 1080),
            timezone: "America/New_York".to_string(),
            language: "en-US,en;q=0.9".to_string(),
            platform: "Win32".to_string(),
            hardware_concurrency: 8,
            device_memory: None,
        }
    }
}

impl Default for TlsFingerprint {
    fn default() -> Self {
        Self::chrome()
    }
}

impl TlsFingerprint {
    pub fn chrome() -> Self {
        Self {
            ja3_hash: "4c4a5d79d5b23c5b76c0016eb48a89d1".to_string(),
            supported_versions: vec!["TLS 1.3".to_string(), "TLS 1.2".to_string()],
            cipher_suites: vec![
                0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc013,
                0xc014, 0x009d, 0x009c, 0x002f, 0x0035,
            ],
            extensions: vec![
                0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009,
                0x000a, 0x000b, 0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011, 0x0012, 0x0013,
                0x0014, 0x0015, 0x0016, 0x0017, 0x0018, 0x0019, 0x001a, 0x001b, 0x001c, 0x001d,
                0x001e, 0x001f, 0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027,
                0x0028, 0x0029, 0x002a, 0x002b, 0x002c, 0x002d, 0x002e, 0x002f, 0x0030, 0x0031,
                0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003a, 0x003b,
                0x003c, 0x003d, 0x003e, 0x003f, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045,
                0x0046, 0x0047,
            ],
            curves: vec![
                "secp256r1".to_string(),
                "secp384r1".to_string(),
                "secp521r1".to_string(),
                "x25519".to_string(),
            ],
        }
    }

    pub fn firefox() -> Self {
        Self {
            ja3_hash: "b6749464770b58d3e2385c7b9f71e7e5".to_string(),
            supported_versions: vec!["TLS 1.3".to_string(), "TLS 1.2".to_string()],
            cipher_suites: vec![
                0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0xcca9, 0xcca8, 0xc013,
                0xc014, 0x009d, 0x009c, 0x002f, 0x0035,
            ],
            extensions: vec![
                0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009,
                0x000a, 0x000b, 0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011, 0x0012, 0x0013,
                0x0014, 0x0015, 0x0016, 0x0017, 0x0018, 0x0019, 0x001a, 0x001b, 0x001c, 0x001d,
                0x001e, 0x001f, 0x0020, 0x0021, 0x0022, 0x0023, 0x0024, 0x0025, 0x0026, 0x0027,
            ],
            curves: vec![
                "secp256r1".to_string(),
                "secp384r1".to_string(),
                "secp521r1".to_string(),
                "x25519".to_string(),
            ],
        }
    }
}

impl StealthConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_jitter(mut self, min_ms: u64, max_ms: u64) -> Self {
        self.jitter_min_ms = min_ms;
        self.jitter_max_ms = max_ms;
        self
    }

    pub fn with_header_rotation(mut self) -> Self {
        self.rotate_headers = true;
        self
    }

    pub fn with_browser(mut self, browser: BrowserFingerprint) -> Self {
        self.browser_fingerprint = browser;
        self
    }

    pub fn with_tls(mut self, tls: TlsFingerprint) -> Self {
        self.tls_fingerprint = tls;
        self
    }

    pub fn as_chrome(self) -> Self {
        self.with_browser(BrowserFingerprint::chrome())
            .with_tls(TlsFingerprint::chrome())
    }

    pub fn as_firefox(self) -> Self {
        self.with_browser(BrowserFingerprint::firefox())
            .with_tls(TlsFingerprint::firefox())
    }

    pub fn random_user_agent(&self) -> String {
        if self.user_agents.is_empty() {
            return "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string();
        }
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.user_agents.len());
        self.user_agents[idx].clone()
    }

    pub fn random_delay(&self) -> Option<Duration> {
        if self.jitter_min_ms == 0 && self.jitter_max_ms == 0 {
            return None;
        }
        let mut rng = rand::thread_rng();
        let min = self.jitter_min_ms.min(self.jitter_max_ms);
        let max = self.jitter_min_ms.max(self.jitter_max_ms);
        let ms = rng.gen_range(min..=max);
        Some(Duration::from_millis(ms))
    }

    pub fn randomize_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = Vec::new();

        if self.rotate_headers {
            let accept_values = [
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
                "*/*",
            ];

            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..accept_values.len());
            headers.push(("Accept", accept_values[idx].to_string()));

            let accept_lang = ["en-US,en;q=0.9", "en-GB,en;q=0.9", "en-US,en;q=0.5"];
            let idx = rng.gen_range(0..accept_lang.len());
            headers.push(("Accept-Language", accept_lang[idx].to_string()));

            headers.push(("Accept-Encoding", "gzip, deflate, br".to_string()));

            headers.push(("DNT", "1".to_string()));

            headers.push((
                "Sec-Ch-Ua",
                "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\""
                    .to_string(),
            ));
            headers.push((
                "Sec-Ch-Ua-Mobile",
                if self.browser_fingerprint.platform.contains("Win") {
                    "?0".to_string()
                } else {
                    "?1".to_string()
                },
            ));
            headers.push((
                "Sec-Ch-Ua-Platform",
                format!("\"{}\"", self.browser_fingerprint.platform),
            ));
            headers.push(("Sec-Fetch-Dest", "document".to_string()));
            headers.push(("Sec-Fetch-Mode", "navigate".to_string()));
            headers.push(("Sec-Fetch-Site", "none".to_string()));
            headers.push(("Sec-Fetch-User", "?1".to_string()));
            headers.push(("Upgrade-Insecure-Requests", "1".to_string()));
        }

        headers
    }

    pub fn generate_webgl_fingerprint(&self) -> String {
        let mut rng = rand::thread_rng();
        let renderer = [
            "Google SwiftShader",
            "ANGLE (NVIDIA, NVIDIA GeForce RTX 3080 Direct3D11 vs_5_0 ps_5_0)",
            "Intel Iris OpenGL Engine",
            "Apple M1",
        ];
        format!(
            "{},{},{}",
            renderer[rng.gen_range(0..renderer.len())],
            self.browser_fingerprint.screen_resolution.0,
            self.browser_fingerprint.screen_resolution.1
        )
    }
}

pub fn default_user_agents() -> Vec<String> {
    vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15".to_string(),
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/120.0.0.0 Safari/537.36".to_string(),
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1".to_string(),
        "Mozilla/5.0 (iPad; CPU OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1".to_string(),
        "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36".to_string(),
    ]
}

pub fn tool_user_agent() -> String {
    format!("Slapper/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stealth_config_default() {
        let config = StealthConfig::default();
        assert!(!config.user_agents.is_empty());
    }

    #[test]
    fn test_random_user_agent() {
        let config = StealthConfig::new();
        let ua = config.random_user_agent();
        assert!(ua.contains("Mozilla"));
    }

    #[test]
    fn test_no_jitter() {
        let config = StealthConfig::new();
        assert!(config.random_delay().is_none());
    }

    #[test]
    fn test_with_jitter() {
        let config = StealthConfig::new().with_jitter(100, 500);
        let delay = config.random_delay().unwrap();
        assert!(delay.as_millis() >= 100);
        assert!(delay.as_millis() <= 500);
    }

    #[test]
    fn test_default_user_agents() {
        let uas = default_user_agents();
        assert!(uas.len() >= 10);
    }

    #[test]
    fn test_tool_user_agent() {
        let ua = tool_user_agent();
        assert!(ua.starts_with("Slapper/"));
    }
}
