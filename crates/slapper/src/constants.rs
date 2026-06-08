//! Centralized constants for Slapper
//!
//! The canonical definitions live in `slapper-core`. This module re-exports
//! them so existing `crate::constants::*` paths continue to work.

pub use slapper_core::constants::*;

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
