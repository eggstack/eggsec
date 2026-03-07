//! Slapper - High-performance security testing toolkit
//!
//! This is the main library crate providing security scanning functionality.

#![allow(clippy::too_many_arguments)] // CLI argument structs require many arguments
#![allow(clippy::vec_init_then_push)] // 194 instances, acceptable for dynamic content
#![allow(clippy::new_without_default)] // Common for builder patterns
#![allow(clippy::derivable_impls)] // Some manual impls for custom formatting
#![allow(clippy::collapsible_if)] // Occasional explicit structure preferred
#![allow(clippy::collapsible_else_if)] // Occasional explicit structure preferred
#![allow(clippy::clone_on_copy)] // Common when &str -> String needed
#![allow(clippy::manual_range_contains)] // Occasional clarity preferred
#![allow(clippy::single_char_add_str)] // Minor readability
#![allow(clippy::should_implement_trait)] // Builder patterns
#![allow(clippy::get_first)] // Minor
#![allow(clippy::manual_find)] // Occasional clarity
#![allow(clippy::manual_ok_err)] // Custom error handling
#![allow(clippy::redundant_closure)] // Occasional specificity needed
#![allow(clippy::unnecessary_to_owned)] // Minor
#![allow(clippy::let_and_return)] // Minor
#![allow(clippy::map_flatten)] // Occasional clarity
#![allow(clippy::len_zero)] // Idiomatic checks
#![allow(clippy::unused_enumerate_index)] // Minor
#![allow(clippy::needless_borrow)] // Minor
#![allow(clippy::single_match)] // Occasional explicit preferred
#![allow(clippy::manual_strip)] // Minor
#![allow(clippy::manual_clamp)] // Minor
#![allow(clippy::double_ended_iterator_last)] // Minor
#![allow(clippy::useless_format)] // Minor
#![allow(clippy::unnecessary_map_or)] // Minor
#![allow(clippy::manual_range_patterns)] // Minor
#![allow(clippy::redundant_locals)] // Minor
#![allow(clippy::op_ref)] // Minor
#![allow(clippy::manual_is_multiple_of)] // Minor
#![allow(clippy::large_enum_variant)] // CLI commands have large variants
#![allow(clippy::iter_next_loop)] // Minor
#![allow(clippy::for_kv_map)] // Minor
#![allow(clippy::if_same_then_else)] // Minor
#![allow(clippy::io_other_error)] // Error handling
#![allow(clippy::useless_vec)] // Minor
#![allow(clippy::single_component_path_imports)] // Minor
#![allow(clippy::upper_case_acronyms)] // Many acronyms in security domain
#![allow(clippy::wrong_self_convention)] // Builder patterns

pub mod cli;
pub mod config;
pub mod constants;
pub mod distributed;
pub mod error;
pub mod fuzzer;
pub mod loadtest;
pub mod logging;
pub mod notify;
pub mod output;
pub mod pipeline;
pub mod proxy;
pub mod recon;
pub mod scanner;
pub mod stress;
pub mod tui;
pub mod utils;
pub mod waf;

#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api", feature = "mcp-server"))]
pub mod tool;

#[cfg(feature = "nse")]
pub mod nse;

#[cfg(feature = "python-plugins")]
pub mod plugin;

#[cfg(feature = "ruby-plugins")]
pub mod ruby;

#[cfg(any(feature = "packet-inspection", feature = "stress-testing"))]
pub mod packet;

#[cfg(feature = "packet-inspection")]
pub mod msf;

#[cfg(not(any(feature = "packet-inspection", feature = "stress-testing")))]
mod msf_stub {
    #![allow(dead_code)]
    use std::io;
    
    pub fn module() -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "msf requires packet-inspection feature"))
    }
}

pub use error::{SlapperError, Result};
pub use config::{SlapperConfig, Scope, load_config, load_scope};
