#![allow(unused_imports)]

pub mod cache;
pub mod formatting;
pub mod http;
pub mod output;
pub mod parsing;
pub mod rate_limiter;
pub mod scope;
pub mod stealth;
pub mod target;
pub mod urlencoding;
pub mod validation;

#[cfg(feature = "stress-testing")]
pub mod privilege;

pub use formatting::{truncate, truncate_simple};
pub use http::{create_http_client, create_insecure_http_client, create_http_client_with_proxy, create_http_client_with_options, create_insecure_client_with_options};
pub use output::{print_json, print_json_compact, print_success, print_error, print_warning, print_info};
pub use parsing::{parse_headers, parse_url_validated};
pub use scope::{check_scope, check_scope_from_url};
pub use target::{extract_target_from_url, extract_host_port, is_ip_address, parse_host_port, normalize_url, extract_domain};

#[cfg(feature = "stress-testing")]
pub use privilege::{is_root, check_privileged, require_root};
