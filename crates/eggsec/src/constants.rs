//! Centralized constants for Eggsec
//!
//! # Re-export shim (intentionally stable)
//!
//! The canonical definitions live in `eggsec-core`. This module re-exports
//! them via `pub use eggsec_core::constants::*` so existing
//! `crate::constants::*` paths continue to work.
//!
//! This file also contains a validation test that ensures `SUPPORTED_WAF_COUNT`
//! matches the actual WAF signature count at compile time.

pub use eggsec_core::constants::*;

#[cfg(test)]
mod tests {
    use super::SUPPORTED_WAF_COUNT;

    #[test]
    fn supported_waf_count_matches_actual() {
        let count = crate::waf::waf_patterns::get_waf_signatures().len();
        assert_eq!(
            count, SUPPORTED_WAF_COUNT,
            "SUPPORTED_WAF_COUNT must match actual detector count"
        );
    }
}
