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
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/eggsec-daemon.sock".into(),
            max_clients: 10,
            default_surface: RuntimeSurface::Unknown,
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
    }
}
