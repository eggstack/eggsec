pub mod approval;
pub mod cancel;
pub mod capabilities;
pub mod daemon_endpoint;
pub mod dispatcher;
pub mod error;
pub mod event;
pub mod ids;
pub mod report;
pub mod request;
pub mod runtime;
pub mod runtime_config;
pub mod runtime_sink;
#[cfg(feature = "schema")]
pub mod schema;
pub mod session;

pub use approval::{ApprovalAnswer, ApprovalGate, ApprovalRegistry};
pub use cancel::race_with_cancel;
pub use capabilities::{RuntimeCapabilities, TaskCapability};
pub use daemon_endpoint::{
    daemon_socket_path_from_env, resolve_daemon_socket_path, DAEMON_SOCKET_ENV_VAR,
    DEFAULT_DAEMON_SOCKET_PATH,
};
pub use error::RuntimeError;
pub use event::{
    ArtifactRef, LogLevel, PolicyPrompt, RuntimeAuditEvent, RuntimeErrorInfo, RuntimeEvent,
    TaskOutcome, TaskProgress, TaskResultEnvelope, TaskStatus,
};
pub use ids::{ClientId, SessionId, TaskId};
pub use report::{RuntimeReportStore, MAX_RETAINED_REPORTS};
pub use request::{LoadTestParams, RunRequest, RuntimeSurface, TaskKind};
pub use runtime::{Runtime, RuntimeTaskExecutor};
pub use runtime_config::{RuntimeConfig, SessionOptions};
pub use runtime_sink::{RuntimeEventReceiver, RuntimeEventSink};
pub use session::{
    RuntimeExecutionContext, RuntimeSession, SessionScope, SessionSnapshot, SessionSummary,
    TaskSnapshot,
};
pub use tokio_util::sync::CancellationToken;
