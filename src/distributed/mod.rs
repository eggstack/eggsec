#![allow(dead_code)]

pub mod worker;
pub mod queue;
pub mod command;
pub mod remote;
pub mod io;


pub use queue::{Task, TaskResult};
pub use command::{RemoteResult, generate_psk};
pub use remote::{RemoteListener, RemoteClient, TlsConfig};

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
