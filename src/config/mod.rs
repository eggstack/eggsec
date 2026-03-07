#![allow(unused_imports)]
#![allow(dead_code)]

mod loader;
mod scope;
mod settings;

pub use loader::{config_dir, load_config, load_scope};
pub use scope::{Scope, ScopeRule, TargetScope};
pub use settings::{
    AllowedWorker, ApiConfig, ApiKeyConfig, HttpConfig, NotificationConfig, OutputConfig,
    ProxyConfigEntry, ReconConfig, RemoteConfig, ScanConfig, ScanProfile, ScheduledScan,
    SlapperConfig, WaybackConfig, WebhookConfig, WebhookEvent,
};

pub const ENV_PREFIX: &str = "SLAPPER_";
