use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimingPreset {
    Paranoid,
    Sneaky,
    Polite,
    Normal,
    Aggressive,
    Insane,
}

impl TimingPreset {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "t0" | "paranoid" | "0" => TimingPreset::Paranoid,
            "t1" | "sneaky" | "1" => TimingPreset::Sneaky,
            "t2" | "polite" | "2" => TimingPreset::Polite,
            "t3" | "normal" | "3" => TimingPreset::Normal,
            "t4" | "aggressive" | "4" => TimingPreset::Aggressive,
            "t5" | "insane" | "5" => TimingPreset::Insane,
            _ => TimingPreset::Normal,
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => TimingPreset::Paranoid,
            1 => TimingPreset::Sneaky,
            2 => TimingPreset::Polite,
            3 => TimingPreset::Normal,
            4 => TimingPreset::Aggressive,
            5 => TimingPreset::Insane,
            _ => TimingPreset::Normal,
        }
    }
}

impl std::fmt::Display for TimingPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimingPreset::Paranoid => write!(f, "paranoid (T0)"),
            TimingPreset::Sneaky => write!(f, "sneaky (T1)"),
            TimingPreset::Polite => write!(f, "polite (T2)"),
            TimingPreset::Normal => write!(f, "normal (T3)"),
            TimingPreset::Aggressive => write!(f, "aggressive (T4)"),
            TimingPreset::Insane => write!(f, "insane (T5)"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimingConfig {
    pub preset: TimingPreset,
    pub min_parallelism: usize,
    pub max_parallelism: usize,
    pub timeout_ms: u64,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
    pub max_rate: Option<u32>,
    pub port_batch_size: usize,
    pub scan_delay_ms: u64,
}

impl TimingConfig {
    pub fn from_preset(preset: TimingPreset) -> Self {
        match preset {
            TimingPreset::Paranoid => TimingConfig {
                preset,
                min_parallelism: 1,
                max_parallelism: 5,
                timeout_ms: 300000,
                retry_count: 5,
                retry_delay_ms: 1000,
                max_rate: Some(1),
                port_batch_size: 1,
                scan_delay_ms: 1000,
            },
            TimingPreset::Sneaky => TimingConfig {
                preset,
                min_parallelism: 5,
                max_parallelism: 15,
                timeout_ms: 60000,
                retry_count: 4,
                retry_delay_ms: 500,
                max_rate: Some(10),
                port_batch_size: 5,
                scan_delay_ms: 200,
            },
            TimingPreset::Polite => TimingConfig {
                preset,
                min_parallelism: 10,
                max_parallelism: 30,
                timeout_ms: 30000,
                retry_count: 3,
                retry_delay_ms: 250,
                max_rate: Some(50),
                port_batch_size: 10,
                scan_delay_ms: 100,
            },
            TimingPreset::Normal => TimingConfig {
                preset,
                min_parallelism: 30,
                max_parallelism: 100,
                timeout_ms: 15000,
                retry_count: 2,
                retry_delay_ms: 100,
                max_rate: Some(200),
                port_batch_size: 25,
                scan_delay_ms: 50,
            },
            TimingPreset::Aggressive => TimingConfig {
                preset,
                min_parallelism: 100,
                max_parallelism: 300,
                timeout_ms: 8000,
                retry_count: 1,
                retry_delay_ms: 50,
                max_rate: Some(1000),
                port_batch_size: 50,
                scan_delay_ms: 10,
            },
            TimingPreset::Insane => TimingConfig {
                preset,
                min_parallelism: 300,
                max_parallelism: 1000,
                timeout_ms: 3000,
                retry_count: 0,
                retry_delay_ms: 0,
                max_rate: None,
                port_batch_size: 100,
                scan_delay_ms: 0,
            },
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self::from_preset(TimingPreset::from_str(s))
    }

    pub fn from_u8(v: u8) -> Self {
        Self::from_preset(TimingPreset::from_u8(v))
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms)
    }

    pub fn retry_delay(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.retry_delay_ms)
    }

    pub fn scan_delay(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.scan_delay_ms)
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self::from_preset(TimingPreset::Normal)
    }
}

pub struct PortPriority;

impl PortPriority {
    pub const CRITICAL_PORTS: [u16; 20] = [
        21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445, 993, 995, 1723, 3306, 3389,
        5900, 8080,
    ];

    pub const HIGH_PORTS: [u16; 100] = [
        13, 37, 42, 67, 68, 69, 79, 81, 88, 95, 106, 109, 110, 111, 113, 119, 123, 135, 137, 138,
        139, 143, 161, 162, 179, 194, 389, 427, 443, 444, 445, 465, 500, 513, 514, 515, 543, 544,
        548, 554, 587, 631, 636, 646, 873, 990, 993, 995, 1025, 1026, 1027, 1028, 1029, 1110, 1433,
        1720, 1723, 1755, 1900, 2000, 2001, 2049, 2121, 2717, 3000, 3128, 3306, 3389, 3986, 4899,
        5000, 5009, 5051, 5060, 5101, 5190, 5357, 5432, 5631, 5666, 5800, 5900, 6000, 6001, 6646,
        7070, 8000, 8008, 8009, 8080, 8081, 8443, 8888, 9100, 9999, 10000, 32768, 49152, 49153,
        49154,
    ];

    pub fn categorize(ports: &[u16]) -> (Vec<u16>, Vec<u16>, Vec<u16>, Vec<u16>) {
        let mut critical = Vec::new();
        let mut high = Vec::new();
        let mut medium = Vec::new();
        let mut low = Vec::new();

        for &port in ports {
            if Self::CRITICAL_PORTS.contains(&port) {
                critical.push(port);
            } else if Self::HIGH_PORTS.contains(&port) {
                high.push(port);
            } else if port <= 1000 {
                medium.push(port);
            } else {
                low.push(port);
            }
        }

        (critical, high, medium, low)
    }

    pub fn get_top_ports(count: usize) -> Vec<u16> {
        Self::CRITICAL_PORTS.iter().take(count).copied().collect()
    }

    pub fn is_common(port: u16) -> bool {
        Self::CRITICAL_PORTS.contains(&port) || Self::HIGH_PORTS.contains(&port)
    }
}

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
}

impl RetryConfig {
    pub fn new(max_retries: u32, initial_backoff_ms: u64) -> Self {
        Self {
            max_retries,
            initial_backoff_ms,
            max_backoff_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }

    pub fn calculate_delay(&self, attempt: u32) -> std::time::Duration {
        let delay = self.initial_backoff_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        std::time::Duration::from_millis(delay.min(self.max_backoff_ms as f64) as u64)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new(3, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_preset_from_str() {
        assert_eq!(TimingPreset::from_str("T0"), TimingPreset::Paranoid);
        assert_eq!(TimingPreset::from_str("paranoid"), TimingPreset::Paranoid);
        assert_eq!(TimingPreset::from_str("T5"), TimingPreset::Insane);
        assert_eq!(TimingPreset::from_str("insane"), TimingPreset::Insane);
        assert_eq!(TimingPreset::from_str("invalid"), TimingPreset::Normal);
    }

    #[test]
    fn test_timing_config_defaults() {
        let config = TimingConfig::default();
        assert_eq!(config.preset, TimingPreset::Normal);
        assert_eq!(config.max_parallelism, 100);
    }

    #[test]
    fn test_port_priority_categorize() {
        let ports = vec![22, 80, 443, 8080, 12345];
        let (critical, _high, _medium, low) = PortPriority::categorize(&ports);
        assert!(critical.contains(&22));
        assert!(critical.contains(&80));
        assert!(critical.contains(&443));
        assert!(critical.contains(&8080));
        assert!(low.contains(&12345));
    }

    #[test]
    fn test_retry_delay() {
        let config = RetryConfig::new(3, 100);
        assert_eq!(config.calculate_delay(0).as_millis(), 100);
        assert_eq!(config.calculate_delay(1).as_millis(), 200);
        assert_eq!(config.calculate_delay(2).as_millis(), 400);
    }
}
