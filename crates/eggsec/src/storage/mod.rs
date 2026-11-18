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
/// moment, and never reaches a wire DTO, a `TaskSnapshot` or a log line. An
/// explicitly named but unset variable is an error, never an anonymous
/// passwordless fallback. `None` retains the no-password behavior.
pub fn resolve_config(
    host: String,
    port: u16,
    database: String,
    username: String,
    password_env: Option<&str>,
    max_connections: u32,
) -> Result<StorageConfig> {
    let password = match password_env {
        Some(var) => std::env::var(var).map_err(|_| {
            crate::error::EggsecError::Config(format!(
                "storage password environment variable '{var}' is not set"
            ))
        })?,
        None => String::new(),
    };
    Ok(StorageConfig {
        host,
        port,
        database,
        username,
        password: SensitiveString::new(password),
        max_connections,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicitly_named_missing_password_variable_fails_closed() {
        let env_name = format!("EGGSEC_STORAGE_MISSING_TEST_{}", std::process::id());
        assert!(std::env::var_os(&env_name).is_none());

        let error = resolve_config(
            "localhost".into(),
            5432,
            "eggsec".into(),
            "postgres".into(),
            Some(&env_name),
            5,
        )
        .expect_err("an explicitly named missing credential must fail");

        assert!(error.to_string().contains(&env_name));
        assert!(!error.to_string().contains("password value"));
    }

    #[test]
    fn absent_password_environment_name_preserves_passwordless_semantics() {
        let config = resolve_config(
            "localhost".into(),
            5432,
            "eggsec".into(),
            "postgres".into(),
            None,
            5,
        )
        .expect("no configured password remains valid");
        assert!(config.password.expose_secret().is_empty());
    }
}
