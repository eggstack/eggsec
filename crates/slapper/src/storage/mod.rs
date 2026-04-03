//! Database storage module
//!
//! Provides persistent storage for scan results, findings, and metadata using PostgreSQL.
//!
//! ## Modules
//!
//! - [`models`] - Database model definitions
//! - [`postgres`] - PostgreSQL connection and operations
//! - [`queries`] - Predefined database queries

pub mod models;
pub mod postgres;
pub mod queries;

use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "slapper".to_string(),
            username: "postgres".to_string(),
            password: "".to_string(),
            max_connections: 10,
        }
    }
}

#[cfg(feature = "database")]
pub async fn init_storage(config: &StorageConfig) -> Result<postgres::Database> {
    postgres::Database::new(config).await
}

#[cfg(not(feature = "database"))]
pub async fn init_storage(_config: &StorageConfig) -> Result<postgres::Database> {
    Err(crate::error::SlapperError::Config(
        "database feature not enabled".to_string(),
    ))
}
