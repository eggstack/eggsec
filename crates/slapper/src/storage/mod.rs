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

use crate::{
    error::Result,
    types::SensitiveString,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: SensitiveString,
    pub max_connections: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "slapper".to_string(),
            username: "postgres".to_string(),
            password: SensitiveString::new(String::new()),
            max_connections: 10,
        }
    }
}

impl Debug for StorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("max_connections", &self.max_connections)
            .finish()
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
