use eggsec_runtime::RuntimeSurface;

/// Configuration for the eggsec daemon.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Unix socket path for client connections.
    pub socket_path: String,
    /// Maximum number of concurrent client connections.
    pub max_clients: usize,
    /// Default execution surface for sessions created without an explicit surface.
    pub default_surface: RuntimeSurface,
    /// Directory for persistent state (sessions, audit log).
    /// Defaults to `~/.local/share/eggsec/daemon/` if None.
    pub data_dir: Option<String>,
    /// Whether to persist session snapshots at lifecycle points.
    pub enable_persistence: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/eggsec-daemon.sock".into(),
            max_clients: 10,
            default_surface: RuntimeSurface::Unknown,
            data_dir: None,
            enable_persistence: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_socket_path() {
        let config = DaemonConfig::default();
        assert_eq!(config.socket_path, "/tmp/eggsec-daemon.sock");
    }

    #[test]
    fn default_config_max_clients() {
        let config = DaemonConfig::default();
        assert_eq!(config.max_clients, 10);
    }

    #[test]
    fn default_config_surface() {
        let config = DaemonConfig::default();
        assert_eq!(config.default_surface, RuntimeSurface::Unknown);
    }

    #[test]
    fn config_clone() {
        let config = DaemonConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.socket_path, config.socket_path);
        assert_eq!(cloned.max_clients, config.max_clients);
        assert_eq!(cloned.default_surface, config.default_surface);
        assert_eq!(cloned.data_dir, config.data_dir);
        assert_eq!(cloned.enable_persistence, config.enable_persistence);
    }

    #[test]
    fn default_config_persistence() {
        let config = DaemonConfig::default();
        assert!(config.enable_persistence);
        assert!(config.data_dir.is_none());
    }

    #[test]
    fn config_with_custom_data_dir() {
        let config = DaemonConfig {
            data_dir: Some("/custom/path".into()),
            enable_persistence: false,
            ..Default::default()
        };
        assert_eq!(config.data_dir.as_deref(), Some("/custom/path"));
        assert!(!config.enable_persistence);
    }
}
