//! Domain executor adapters for the dispatch registry.
//!
//! Each executor implements [`OperationExecutor`] to handle a group of
//! related operations. The executors delegate to existing domain functions
//! in `dispatch/scanner.rs`, `dispatch/recon.rs`, etc., without
//! reimplementing logic.

pub mod fuzz;
pub mod network;
pub mod recon;
pub mod registry;
pub mod scanner;
pub mod waf;

#[cfg(feature = "db-pentest")]
pub mod db_pentest;
#[cfg(feature = "nse")]
pub mod nse;

use registry::ExecutorRegistry;

/// Build the default executor registry with all compiled-in executors.
///
/// Feature-gated executors are only registered when their feature is enabled.
pub fn build_default_registry() -> ExecutorRegistry {
    let mut reg = ExecutorRegistry::new();

    // Always-compiled executors
    reg.register(Box::new(scanner::ScannerExecutor));
    reg.register(Box::new(recon::ReconExecutor));
    reg.register(Box::new(waf::WafExecutor));
    reg.register(Box::new(fuzz::FuzzExecutor));
    reg.register(Box::new(network::NetworkExecutor));

    // Feature-gated executors
    #[cfg(feature = "nse")]
    reg.register(Box::new(nse::NseExecutor));

    #[cfg(feature = "db-pentest")]
    reg.register(Box::new(db_pentest::DbPentestExecutor));

    reg
}
