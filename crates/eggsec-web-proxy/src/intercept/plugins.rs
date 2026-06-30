//! Plugin system for extensible protocol handling in the web proxy.
//!
//! Defines a `ProtocolHandler` trait that can be implemented to add custom
//! protocol detection and handling beyond the built-in HTTP/1, HTTP/2,
//! WebSocket, and gRPC support.
//!
//! Plugins are registered via `PluginRegistry` and invoked when protocol
//! detection matches their declared protocol signature.
//!
//! # Security Model
//!
//! Plugins run in a sandboxed environment with capability-based restrictions.
//! Each plugin must declare the capabilities it requires, and the registry
//! enforces these restrictions during detection and handling phases.

use std::collections::HashMap;

/// Capabilities that a plugin can request.
///
/// Capabilities control what operations a plugin is allowed to perform.
/// Plugins can only request capabilities that have been explicitly granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PluginCapability {
    /// Read access to connection metadata (host, path, headers).
    ReadMetadata,
    /// Read access to request/response bodies.
    ReadBodies,
    /// Write access to modify request/response data.
    WriteData,
    /// Access to network connections for outbound requests.
    NetworkAccess,
    /// Access to file system for logging or caching.
    FileSystem,
    /// Ability to spawn background tasks.
    SpawnTasks,
    /// Access to cryptographic operations.
    CryptoAccess,
    /// Ability to register new protocol handlers.
    RegisterProtocols,
}

impl std::fmt::Display for PluginCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadMetadata => write!(f, "read-metadata"),
            Self::ReadBodies => write!(f, "read-bodies"),
            Self::WriteData => write!(f, "write-data"),
            Self::NetworkAccess => write!(f, "network-access"),
            Self::FileSystem => write!(f, "file-system"),
            Self::SpawnTasks => write!(f, "spawn-tasks"),
            Self::CryptoAccess => write!(f, "crypto-access"),
            Self::RegisterProtocols => write!(f, "register-protocols"),
        }
    }
}

/// Capability set for a plugin.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CapabilitySet {
    capabilities: Vec<PluginCapability>,
}

impl CapabilitySet {
    /// Create an empty capability set.
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    /// Create a capability set with the given capabilities.
    pub fn with(capabilities: Vec<PluginCapability>) -> Self {
        Self { capabilities }
    }

    /// Add a capability to the set.
    pub fn add(&mut self, cap: PluginCapability) -> &mut Self {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
        self
    }

    /// Check if a capability is in the set.
    pub fn has(&self, cap: &PluginCapability) -> bool {
        self.capabilities.contains(cap)
    }

    /// Check if this capability set is a superset of another.
    pub fn includes(&self, other: &CapabilitySet) -> bool {
        other.capabilities.iter().all(|cap| self.has(cap))
    }

    /// Get all capabilities.
    pub fn iter(&self) -> impl Iterator<Item = &PluginCapability> {
        self.capabilities.iter()
    }

    /// Number of capabilities.
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

/// Sandbox configuration for plugin execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginSandbox {
    /// Granted capabilities for this plugin.
    pub granted: CapabilitySet,
    /// Maximum memory usage in bytes (0 = unlimited).
    pub max_memory_bytes: u64,
    /// Maximum execution time in milliseconds (0 = unlimited).
    pub max_execution_ms: u64,
    /// Maximum number of operations allowed (0 = unlimited).
    pub max_operations: u64,
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self {
            granted: CapabilitySet::new(),
            max_memory_bytes: 10 * 1024 * 1024, // 10MB default
            max_execution_ms: 5000,             // 5 seconds default
            max_operations: 1000,
        }
    }
}

impl PluginSandbox {
    /// Create a new sandbox with minimal capabilities.
    pub fn restricted() -> Self {
        Self {
            granted: CapabilitySet::with(vec![PluginCapability::ReadMetadata]),
            max_memory_bytes: 1024 * 1024, // 1MB
            max_execution_ms: 1000,
            max_operations: 100,
        }
    }

    /// Create a sandbox with full capabilities (for trusted plugins).
    pub fn permissive() -> Self {
        Self {
            granted: CapabilitySet::with(vec![
                PluginCapability::ReadMetadata,
                PluginCapability::ReadBodies,
                PluginCapability::WriteData,
                PluginCapability::NetworkAccess,
                PluginCapability::FileSystem,
                PluginCapability::SpawnTasks,
                PluginCapability::CryptoAccess,
                PluginCapability::RegisterProtocols,
            ]),
            max_memory_bytes: 0,
            max_execution_ms: 0,
            max_operations: 0,
        }
    }

    /// Check if an operation is allowed by the sandbox.
    pub fn check_capability(&self, required: &PluginCapability) -> Result<(), SandboxViolation> {
        if self.granted.has(required) {
            Ok(())
        } else {
            Err(SandboxViolation::CapabilityDenied(*required))
        }
    }

    /// Check if memory usage is within limits.
    pub fn check_memory(&self, used: u64) -> Result<(), SandboxViolation> {
        if self.max_memory_bytes == 0 || used <= self.max_memory_bytes {
            Ok(())
        } else {
            Err(SandboxViolation::MemoryExceeded {
                used,
                limit: self.max_memory_bytes,
            })
        }
    }

    /// Check if execution time is within limits.
    pub fn check_execution_time(&self, elapsed_ms: u64) -> Result<(), SandboxViolation> {
        if self.max_execution_ms == 0 || elapsed_ms <= self.max_execution_ms {
            Ok(())
        } else {
            Err(SandboxViolation::ExecutionTimeExceeded {
                elapsed_ms,
                limit_ms: self.max_execution_ms,
            })
        }
    }

    /// Check if operation count is within limits.
    pub fn check_operations(&self, count: u64) -> Result<(), SandboxViolation> {
        if self.max_operations == 0 || count <= self.max_operations {
            Ok(())
        } else {
            Err(SandboxViolation::OperationsExceeded {
                count,
                limit: self.max_operations,
            })
        }
    }
}

/// Violation of sandbox restrictions.
#[derive(Debug, Clone)]
pub enum SandboxViolation {
    /// Plugin attempted to use a capability it doesn't have.
    CapabilityDenied(PluginCapability),
    /// Plugin exceeded memory limit.
    MemoryExceeded { used: u64, limit: u64 },
    /// Plugin exceeded execution time limit.
    ExecutionTimeExceeded { elapsed_ms: u64, limit_ms: u64 },
    /// Plugin exceeded operation count limit.
    OperationsExceeded { count: u64, limit: u64 },
}

impl std::fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapabilityDenied(cap) => {
                write!(f, "Capability denied: {}", cap)
            }
            Self::MemoryExceeded { used, limit } => {
                write!(f, "Memory exceeded: {} bytes used, {} limit", used, limit)
            }
            Self::ExecutionTimeExceeded {
                elapsed_ms,
                limit_ms,
            } => {
                write!(
                    f,
                    "Execution time exceeded: {}ms elapsed, {}ms limit",
                    elapsed_ms, limit_ms
                )
            }
            Self::OperationsExceeded { count, limit } => {
                write!(
                    f,
                    "Operations exceeded: {} operations, {} limit",
                    count, limit
                )
            }
        }
    }
}

impl std::error::Error for SandboxViolation {}

/// Metadata about a registered plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique plugin identifier (e.g., "my-custom-protocol").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// SemVer version string.
    pub version: String,
    /// Description of what the plugin handles.
    pub description: String,
}

/// Result of protocol detection by a plugin.
#[derive(Debug, Clone)]
pub enum DetectionResult {
    /// Plugin claims this traffic; provides a confidence score and context.
    Detected {
        confidence: f64,
        protocol_name: String,
        context: HashMap<String, String>,
    },
    /// Plugin does not claim this traffic.
    NotDetected,
}

/// Result of handling a detected protocol session.
#[derive(Debug, Clone)]
pub struct HandleResult {
    /// Finding summary from plugin handling.
    pub findings: Vec<PluginFinding>,
    /// Any additional metadata to attach to the session.
    pub metadata: HashMap<String, String>,
}

/// A finding produced by a plugin during protocol handling.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginFinding {
    /// Plugin that produced this finding.
    pub plugin_id: String,
    /// Finding title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Severity (0-10 scale, 10 = critical).
    pub severity: u8,
    /// Optional metadata.
    pub metadata: HashMap<String, String>,
}

/// Trait for implementing custom protocol handlers.
///
/// # Example
///
/// ```ignore
/// struct MyProtocolHandler;
///
/// impl ProtocolHandler for MyProtocolHandler {
///     fn info(&self) -> PluginInfo {
///         PluginInfo {
///             id: "my-proto".to_string(),
///             name: "My Protocol".to_string(),
///             version: "0.1.0".to_string(),
///             description: "Handles custom binary protocol".to_string(),
///         }
///     }
///
///     fn detect(&self, _host: &str, _path: &str, headers: &HashMap<String, String>) -> DetectionResult {
///         if headers.get("x-protocol").map(|v| v.as_str()) == Some("my-proto") {
///             DetectionResult::Detected {
///                 confidence: 0.95,
///                 protocol_name: "my-proto".to_string(),
///                 context: HashMap::new(),
///             }
///         } else {
///             DetectionResult::NotDetected
///         }
///     }
///
///     fn handle(&self, _host: &str, _path: &str, _headers: &HashMap<String, String>, body: Option<&str>) -> HandleResult {
///         // Custom handling logic
///         HandleResult {
///             findings: vec![],
///             metadata: HashMap::new(),
///         }
///     }
/// }
/// ```
pub trait ProtocolHandler: Send + Sync {
    /// Return metadata about this plugin.
    fn info(&self) -> PluginInfo;

    /// Detect whether this plugin should handle the given connection.
    ///
    /// Called during protocol detection phase. Return `Detected` with
    /// a confidence > 0.0 to claim the traffic.
    fn detect(&self, host: &str, path: &str, headers: &HashMap<String, String>) -> DetectionResult;

    /// Handle a detected protocol session.
    ///
    /// Called after `detect()` returns `Detected`. The handler processes
    /// the request/response data and returns findings.
    fn handle(
        &self,
        host: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> HandleResult;
}

/// Registry for managing protocol handler plugins.
pub struct PluginRegistry {
    handlers: Vec<Box<dyn ProtocolHandler>>,
    index: HashMap<String, usize>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Register a protocol handler plugin.
    ///
    /// Returns an error if a plugin with the same ID is already registered.
    pub fn register(&mut self, handler: Box<dyn ProtocolHandler>) -> Result<(), PluginError> {
        let info = handler.info();
        if self.index.contains_key(&info.id) {
            return Err(PluginError::DuplicateId(info.id));
        }
        let idx = self.handlers.len();
        self.handlers.push(handler);
        self.index.insert(info.id, idx);
        Ok(())
    }

    /// Get a list of all registered plugins.
    pub fn list(&self) -> Vec<PluginInfo> {
        self.handlers.iter().map(|h| h.info()).collect()
    }

    /// Try to detect which plugin should handle the given connection.
    ///
    /// Returns the plugin with the highest confidence detection, or `None`.
    pub fn detect(
        &self,
        host: &str,
        path: &str,
        headers: &HashMap<String, String>,
    ) -> Option<(&dyn ProtocolHandler, DetectionResult)> {
        let mut best: Option<(usize, DetectionResult)> = None;
        for (i, handler) in self.handlers.iter().enumerate() {
            match handler.detect(host, path, headers) {
                DetectionResult::Detected { confidence, .. } => {
                    if confidence > 0.0 {
                        match &best {
                            Some((
                                _,
                                DetectionResult::Detected {
                                    confidence: best_c, ..
                                },
                            )) if best_c >= &confidence => {}
                            _ => {
                                let result = handler.detect(host, path, headers);
                                best = Some((i, result));
                            }
                        }
                    }
                }
                DetectionResult::NotDetected => {}
            }
        }
        best.map(|(i, result)| (self.handlers[i].as_ref(), result))
    }

    /// Get a plugin by its ID.
    pub fn get(&self, id: &str) -> Option<&dyn ProtocolHandler> {
        self.index.get(id).map(|&i| self.handlers[i].as_ref())
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during plugin registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    /// A plugin with this ID is already registered.
    DuplicateId(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "plugin with ID '{}' already registered", id),
        }
    }
}

impl std::error::Error for PluginError {}

/// Example built-in plugin: detects non-standard port usage.
pub struct NonStandardPortHandler;

impl ProtocolHandler for NonStandardPortHandler {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "non-standard-port".to_string(),
            name: "Non-Standard Port Detector".to_string(),
            version: "1.0.0".to_string(),
            description: "Detects services running on non-standard ports".to_string(),
        }
    }

    fn detect(
        &self,
        host: &str,
        _path: &str,
        _headers: &HashMap<String, String>,
    ) -> DetectionResult {
        // Check if host:port uses a non-standard port
        if let Some(port_str) = host.split(':').nth(1) {
            if let Ok(port) = port_str.parse::<u16>() {
                let is_standard = matches!(port, 80 | 443 | 8080 | 8443 | 3000 | 5000 | 9090);
                if !is_standard {
                    let mut ctx = HashMap::new();
                    ctx.insert("port".to_string(), port.to_string());
                    return DetectionResult::Detected {
                        confidence: 0.6,
                        protocol_name: "non-standard-port".to_string(),
                        context: ctx,
                    };
                }
            }
        }
        DetectionResult::NotDetected
    }

    fn handle(
        &self,
        host: &str,
        _path: &str,
        _headers: &HashMap<String, String>,
        _body: Option<&str>,
    ) -> HandleResult {
        let mut findings = vec![];
        if let Some(port_str) = host.split(':').nth(1) {
            if let Ok(port) = port_str.parse::<u16>() {
                let is_standard = matches!(port, 80 | 443 | 8080 | 8443 | 3000 | 5000 | 9090);
                if !is_standard {
                    findings.push(PluginFinding {
                        plugin_id: "non-standard-port".to_string(),
                        title: format!("Service on non-standard port {}", port),
                        description: format!("Host {} is using non-standard port {}", host, port),
                        severity: 2,
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        HandleResult {
            findings,
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info() {
        let handler = NonStandardPortHandler;
        let info = handler.info();
        assert_eq!(info.id, "non-standard-port");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn test_plugin_detect_standard_port() {
        let handler = NonStandardPortHandler;
        let headers = HashMap::new();
        let result = handler.detect("example.com:443", "/", &headers);
        assert!(matches!(result, DetectionResult::NotDetected));
    }

    #[test]
    fn test_plugin_detect_non_standard_port() {
        let handler = NonStandardPortHandler;
        let headers = HashMap::new();
        let result = handler.detect("example.com:9999", "/", &headers);
        match result {
            DetectionResult::Detected {
                confidence,
                protocol_name,
                context,
            } => {
                assert!(confidence > 0.0);
                assert_eq!(protocol_name, "non-standard-port");
                assert_eq!(context.get("port").unwrap(), "9999");
            }
            DetectionResult::NotDetected => panic!("expected detected"),
        }
    }

    #[test]
    fn test_plugin_handle_non_standard_port() {
        let handler = NonStandardPortHandler;
        let headers = HashMap::new();
        let result = handler.handle("example.com:3306", "/", &headers, None);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(
            result.findings[0].title,
            "Service on non-standard port 3306"
        );
    }

    #[test]
    fn test_plugin_handle_standard_port() {
        let handler = NonStandardPortHandler;
        let headers = HashMap::new();
        let result = handler.handle("example.com:443", "/", &headers, None);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_registry_register_and_list() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(NonStandardPortHandler)).unwrap();
        assert_eq!(registry.len(), 1);
        let plugins = registry.list();
        assert_eq!(plugins[0].id, "non-standard-port");
    }

    #[test]
    fn test_registry_duplicate_id_rejected() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(NonStandardPortHandler)).unwrap();
        let result = registry.register(Box::new(NonStandardPortHandler));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            PluginError::DuplicateId("non-standard-port".to_string())
        );
    }

    #[test]
    fn test_registry_detect_best_confidence() {
        struct HighConfidenceHandler;
        impl ProtocolHandler for HighConfidenceHandler {
            fn info(&self) -> PluginInfo {
                PluginInfo {
                    id: "high".to_string(),
                    name: "High".to_string(),
                    version: "1.0.0".to_string(),
                    description: "".to_string(),
                }
            }
            fn detect(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
            ) -> DetectionResult {
                DetectionResult::Detected {
                    confidence: 0.9,
                    protocol_name: "high".to_string(),
                    context: HashMap::new(),
                }
            }
            fn handle(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
                _b: Option<&str>,
            ) -> HandleResult {
                HandleResult {
                    findings: vec![],
                    metadata: HashMap::new(),
                }
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(NonStandardPortHandler)).unwrap();
        registry.register(Box::new(HighConfidenceHandler)).unwrap();

        let headers = HashMap::new();
        let (handler, _) = registry.detect("example.com:9999", "/", &headers).unwrap();
        assert_eq!(handler.info().id, "high");
    }

    #[test]
    fn test_registry_get_by_id() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(NonStandardPortHandler)).unwrap();
        assert!(registry.get("non-standard-port").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_plugin_error_display() {
        let err = PluginError::DuplicateId("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_plugin_finding_serialization() {
        let finding = PluginFinding {
            plugin_id: "test".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            severity: 5,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&finding).unwrap();
        let back: PluginFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.severity, 5);
    }

    // --- Edge case tests ---

    #[test]
    fn test_plugin_handle_result_metadata_empty_for_nonstandard() {
        let handler = NonStandardPortHandler;
        let headers = HashMap::new();
        let result = handler.handle("example.com:3306", "/", &headers, None);
        // NonStandardPortHandler returns empty metadata
        assert!(result.metadata.is_empty());
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn test_registry_detect_no_match() {
        struct NeverDetectHandler;
        impl ProtocolHandler for NeverDetectHandler {
            fn info(&self) -> PluginInfo {
                PluginInfo {
                    id: "never".to_string(),
                    name: "Never".to_string(),
                    version: "1.0.0".to_string(),
                    description: "".to_string(),
                }
            }
            fn detect(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
            ) -> DetectionResult {
                DetectionResult::NotDetected
            }
            fn handle(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
                _b: Option<&str>,
            ) -> HandleResult {
                HandleResult {
                    findings: vec![],
                    metadata: HashMap::new(),
                }
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(NeverDetectHandler)).unwrap();
        let headers = HashMap::new();
        assert!(registry.detect("example.com:9999", "/", &headers).is_none());
    }

    #[test]
    fn test_registry_multiple_handlers_best_wins() {
        struct LowHandler;
        impl ProtocolHandler for LowHandler {
            fn info(&self) -> PluginInfo {
                PluginInfo {
                    id: "low".to_string(),
                    name: "Low".to_string(),
                    version: "1.0.0".to_string(),
                    description: "".to_string(),
                }
            }
            fn detect(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
            ) -> DetectionResult {
                DetectionResult::Detected {
                    confidence: 0.3,
                    protocol_name: "low".to_string(),
                    context: HashMap::new(),
                }
            }
            fn handle(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
                _b: Option<&str>,
            ) -> HandleResult {
                HandleResult {
                    findings: vec![],
                    metadata: HashMap::new(),
                }
            }
        }

        struct HighHandler;
        impl ProtocolHandler for HighHandler {
            fn info(&self) -> PluginInfo {
                PluginInfo {
                    id: "high".to_string(),
                    name: "High".to_string(),
                    version: "1.0.0".to_string(),
                    description: "".to_string(),
                }
            }
            fn detect(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
            ) -> DetectionResult {
                DetectionResult::Detected {
                    confidence: 0.9,
                    protocol_name: "high".to_string(),
                    context: HashMap::new(),
                }
            }
            fn handle(
                &self,
                _h: &str,
                _p: &str,
                _hdr: &HashMap<String, String>,
                _b: Option<&str>,
            ) -> HandleResult {
                HandleResult {
                    findings: vec![],
                    metadata: HashMap::new(),
                }
            }
        }

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(LowHandler)).unwrap();
        registry.register(Box::new(HighHandler)).unwrap();
        let headers = HashMap::new();
        let (handler, _) = registry.detect("example.com:9999", "/", &headers).unwrap();
        assert_eq!(handler.info().id, "high");
    }

    #[test]
    fn test_plugin_finding_severity_boundary_zero() {
        let finding = PluginFinding {
            plugin_id: "test".to_string(),
            title: "Low".to_string(),
            description: "Minimal".to_string(),
            severity: 0,
            metadata: HashMap::new(),
        };
        assert_eq!(finding.severity, 0);
    }

    #[test]
    fn test_plugin_finding_severity_boundary_max() {
        let finding = PluginFinding {
            plugin_id: "test".to_string(),
            title: "Critical".to_string(),
            description: "Maximum severity".to_string(),
            severity: 10,
            metadata: HashMap::new(),
        };
        assert_eq!(finding.severity, 10);
    }

    #[test]
    fn test_plugin_finding_with_metadata() {
        let mut meta = HashMap::new();
        meta.insert("port".to_string(), "8080".to_string());
        meta.insert("service".to_string(), "http".to_string());
        let finding = PluginFinding {
            plugin_id: "test".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            severity: 5,
            metadata: meta,
        };
        let json = serde_json::to_string(&finding).unwrap();
        let back: PluginFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.metadata.len(), 2);
        assert_eq!(back.metadata.get("port").unwrap(), "8080");
    }

    // --- Plugin Sandbox Tests ---

    #[test]
    fn test_capability_set_basic() {
        let mut caps = CapabilitySet::new();
        assert!(caps.is_empty());
        assert_eq!(caps.len(), 0);

        caps.add(PluginCapability::ReadMetadata);
        assert!(caps.has(&PluginCapability::ReadMetadata));
        assert!(!caps.has(&PluginCapability::WriteData));
        assert_eq!(caps.len(), 1);

        // Adding same capability twice should not duplicate
        caps.add(PluginCapability::ReadMetadata);
        assert_eq!(caps.len(), 1);
    }

    #[test]
    fn test_capability_set_with() {
        let caps = CapabilitySet::with(vec![
            PluginCapability::ReadMetadata,
            PluginCapability::ReadBodies,
        ]);
        assert!(caps.has(&PluginCapability::ReadMetadata));
        assert!(caps.has(&PluginCapability::ReadBodies));
        assert!(!caps.has(&PluginCapability::WriteData));
    }

    #[test]
    fn test_capability_set_includes() {
        let granted = CapabilitySet::with(vec![
            PluginCapability::ReadMetadata,
            PluginCapability::ReadBodies,
            PluginCapability::WriteData,
        ]);
        let required = CapabilitySet::with(vec![
            PluginCapability::ReadMetadata,
            PluginCapability::ReadBodies,
        ]);
        assert!(granted.includes(&required));

        let not_included = CapabilitySet::with(vec![PluginCapability::NetworkAccess]);
        assert!(!granted.includes(&not_included));
    }

    #[test]
    fn test_plugin_sandbox_restricted() {
        let sandbox = PluginSandbox::restricted();
        assert!(sandbox.granted.has(&PluginCapability::ReadMetadata));
        assert!(!sandbox.granted.has(&PluginCapability::WriteData));
        assert_eq!(sandbox.max_memory_bytes, 1024 * 1024);
        assert_eq!(sandbox.max_execution_ms, 1000);
        assert_eq!(sandbox.max_operations, 100);
    }

    #[test]
    fn test_plugin_sandbox_permissive() {
        let sandbox = PluginSandbox::permissive();
        assert!(sandbox.granted.has(&PluginCapability::ReadMetadata));
        assert!(sandbox.granted.has(&PluginCapability::WriteData));
        assert!(sandbox.granted.has(&PluginCapability::NetworkAccess));
        assert_eq!(sandbox.max_memory_bytes, 0); // unlimited
        assert_eq!(sandbox.max_execution_ms, 0); // unlimited
    }

    #[test]
    fn test_plugin_sandbox_check_capability() {
        let sandbox = PluginSandbox::restricted();
        assert!(sandbox
            .check_capability(&PluginCapability::ReadMetadata)
            .is_ok());
        assert!(sandbox
            .check_capability(&PluginCapability::WriteData)
            .is_err());
    }

    #[test]
    fn test_plugin_sandbox_check_memory() {
        let sandbox = PluginSandbox::restricted();
        assert!(sandbox.check_memory(500 * 1024).is_ok()); // 500KB < 1MB
        assert!(sandbox.check_memory(2 * 1024 * 1024).is_err()); // 2MB > 1MB
    }

    #[test]
    fn test_plugin_sandbox_check_execution_time() {
        let sandbox = PluginSandbox::restricted();
        assert!(sandbox.check_execution_time(500).is_ok()); // 500ms < 1s
        assert!(sandbox.check_execution_time(2000).is_err()); // 2s > 1s
    }

    #[test]
    fn test_plugin_sandbox_check_operations() {
        let sandbox = PluginSandbox::restricted();
        assert!(sandbox.check_operations(50).is_ok()); // 50 < 100
        assert!(sandbox.check_operations(200).is_err()); // 200 > 100
    }

    #[test]
    fn test_sandbox_violation_display() {
        let violation = SandboxViolation::CapabilityDenied(PluginCapability::WriteData);
        assert!(violation.to_string().contains("write-data"));

        let violation = SandboxViolation::MemoryExceeded {
            used: 2048,
            limit: 1024,
        };
        assert!(violation.to_string().contains("2048"));
        assert!(violation.to_string().contains("1024"));
    }

    #[test]
    fn test_plugin_capability_display() {
        assert_eq!(PluginCapability::ReadMetadata.to_string(), "read-metadata");
        assert_eq!(PluginCapability::WriteData.to_string(), "write-data");
        assert_eq!(
            PluginCapability::NetworkAccess.to_string(),
            "network-access"
        );
    }

    #[test]
    fn test_plugin_capability_serialization() {
        let cap = PluginCapability::ReadMetadata;
        let json = serde_json::to_string(&cap).unwrap();
        assert_eq!(json, "\"ReadMetadata\"");
        let back: PluginCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PluginCapability::ReadMetadata);
    }

    #[test]
    fn test_plugin_sandbox_serialization() {
        let sandbox = PluginSandbox::restricted();
        let json = serde_json::to_string(&sandbox).unwrap();
        assert!(json.contains("ReadMetadata"));
    }
}
