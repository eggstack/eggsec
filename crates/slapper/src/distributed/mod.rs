//! Distributed scanning module
//!
//! Provides infrastructure for distributing scanning tasks across
//! multiple worker nodes for parallel execution.
//!
//! ## Key Components
//!
//! - [`RemoteClient`] - Client for connecting to coordinator
//! - [`RemoteListener`] - Coordinator server for accepting workers
//! - [`Task`] - Individual scanning task definition
//! - [`TaskResult`] - Result from worker task execution
//!
//! ## Usage
//!
//! ### Starting a Coordinator
//!
//! ```rust,no_run
//! use slapper::distributed::RemoteListener;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let listener = RemoteListener::new("my-secret-psk".to_string());
//! listener.start(9000).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Executing on a Worker
//!
//! ```rust,no_run
//! use slapper::distributed::RemoteClient;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let client = RemoteClient::new("my-secret-psk".to_string());
//! let result = client.execute(
//!     "worker-host",
//!     9000,
//!     vec!["slapper".to_string(), "scan-ports".to_string(), "example.com".to_string()],
//!     Some(300),  // timeout
//! ).await?;
//!
//! println!("{}", result.output);
//! # Ok(())
//! # }
//! ```

pub mod command;
pub mod io;
pub mod queue;
pub mod remote;
pub mod worker;

pub use command::{generate_psk, RemoteResult};
pub use queue::{Task, TaskResult};
pub use remote::{RemoteClient, RemoteListener, TlsConfig};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    PortScan,
    ServiceFingerprint,
    EndpointDiscovery,
    Fuzz,
    WafTest,
    LoadTest,
    Recon,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::PortScan => write!(f, "PortScan"),
            TaskType::ServiceFingerprint => write!(f, "ServiceFingerprint"),
            TaskType::EndpointDiscovery => write!(f, "EndpointDiscovery"),
            TaskType::Fuzz => write!(f, "Fuzz"),
            TaskType::WafTest => write!(f, "WafTest"),
            TaskType::LoadTest => write!(f, "LoadTest"),
            TaskType::Recon => write!(f, "Recon"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistration {
    pub worker_id: String,
    pub hostname: String,
    pub capabilities: Vec<TaskType>,
    pub max_concurrency: usize,
    pub status: WorkerStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub worker_id: String,
    pub status: WorkerStatus,
    pub current_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub cpu_usage: f32,
    pub memory_usage: f32,
}
