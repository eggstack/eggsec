mod api;
mod auth;
#[cfg(feature = "c2")]
mod c2_worker;
#[cfg(feature = "db-pentest")]
mod db_pentest;
#[cfg(feature = "web-proxy")]
mod intercept_worker;
mod fuzzer;
mod network;
mod recon;
mod runner;
mod scanner;
mod security;

pub use runner::{TaskConfig, TaskResult, TaskRunner, TracerouteHopResult};

use tokio::sync::mpsc;

/// Send a progress update, logging on channel failure instead of propagating.
pub(crate) async fn send_progress(tx: &mpsc::Sender<(u64, u64)>, done: u64, total: u64) {
    if let Err(e) = tx.send((done, total)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
}

/// Send a task result, logging on channel failure instead of propagating.
pub(crate) async fn send_result(tx: &mpsc::Sender<TaskResult>, result: TaskResult) {
    if let Err(e) = tx.send(result).await {
        tracing::warn!("Failed to send result: {}", e);
    }
}
