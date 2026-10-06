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

use crate::{error::Result, types::SensitiveString};
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
            database: "eggsec".to_string(),
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

/// Build a [`StorageConfig`] from connection fields plus the *name* of an
/// environment variable holding the password.
///
/// Shared by the canonical executor and the `eggsec storage` CLI so the two
/// cannot disagree about how a credential is supplied. `password_env` carries
/// a variable name, never a secret: the value is read here, at the last
/// moment, and never reaches a wire DTO, a `TaskSnapshot` or a log line.
pub fn resolve_config(
    host: String,
    port: u16,
    database: String,
    username: String,
    password_env: Option<&str>,
    max_connections: u32,
) -> StorageConfig {
    let password = match password_env {
        Some(var) => std::env::var(var).unwrap_or_else(|e| {
            tracing::warn!(
                env_var = var,
                error = %e,
                "storage password_env is unset; connecting without a password"
            );
            String::new()
        }),
        None => String::new(),
    };
    StorageConfig {
        host,
        port,
        database,
        username,
        password: SensitiveString::new(password),
        max_connections,
    }
}

#[cfg(feature = "database")]
pub async fn init_storage(config: &StorageConfig) -> Result<postgres::Database> {
    postgres::Database::new(config).await
}

#[cfg(not(feature = "database"))]
pub async fn init_storage(_config: &StorageConfig) -> Result<postgres::Database> {
    Err(crate::error::EggsecError::Config(
        "database feature not enabled".to_string(),
    ))
}
