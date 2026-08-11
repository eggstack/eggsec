//! Fuzzer configuration types
//!
//! These are plain data structs without clap derives, used by both the CLI
//! (via conversion from clap args) and the Python bindings (constructed directly).

use crate::types::CommonHttpArgs;

/// Fuzzing mode: sequential (one-by-one), burst (concurrent), adaptive (auto-adjusts rate)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FuzzMode {
    #[default]
    Sequential,
    Burst,
    Adaptive,
}

impl std::fmt::Display for FuzzMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuzzMode::Sequential => write!(f, "sequential"),
            FuzzMode::Burst => write!(f, "burst"),
            FuzzMode::Adaptive => write!(f, "adaptive"),
        }
    }
}

/// Plain fuzzing configuration (no clap derives)
#[derive(Debug, Clone)]
pub struct FuzzConfig {
    pub url: String,
    pub payload_type: String,
    pub mode: FuzzMode,
    pub mutate: bool,
    pub mutation_count: usize,
    pub grammar_fuzz: bool,
    pub grammar_type: Option<String>,
    pub adaptive_rate: bool,
    pub session: bool,
    pub diffing: bool,
    pub capture_baseline: bool,
    pub enhanced_redos: bool,
    pub waf_fingerprint: bool,
    pub chaining: bool,
    pub chain_file: Option<String>,
    pub method: String,
    pub param: Option<String>,
    pub concurrency: usize,
    pub timeout: u64,
    pub json: bool,
    pub output: Option<String>,
    pub verbose: bool,
    pub quiet: bool,
    pub format: Option<crate::types::OutputFormat>,
    pub target: Option<String>,
    pub jwt_token: Option<String>,
    pub oauth_issuer: Option<String>,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub idor_base_id: Option<String>,
    pub idor_user_ids: Option<String>,
    pub ssti_param: Option<String>,
    pub graphql_introspection: bool,
    pub graphql_depth_bypass: bool,
    pub graphql_alias_overload: bool,
    pub oauth_redirect: bool,
    pub oauth_scope: bool,
    pub oauth_state: bool,
    pub oauth_grant: bool,
    pub schema: Option<String>,
    pub discover_only: bool,
    pub auto_discover_schema: bool,
    pub calibrate: bool,
    pub fc: Option<String>,
    pub fs: Option<String>,
    pub fw: Option<String>,
    pub fl: Option<String>,
    pub ft: Option<u64>,
    pub fr: Option<String>,
    pub common: CommonHttpArgs,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            payload_type: "all".to_string(),
            mode: FuzzMode::default(),
            mutate: false,
            mutation_count: 3,
            grammar_fuzz: false,
            grammar_type: None,
            adaptive_rate: false,
            session: false,
            diffing: false,
            capture_baseline: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            method: "GET".to_string(),
            param: None,
            concurrency: 10,
            timeout: 30,
            json: false,
            output: None,
            verbose: false,
            quiet: false,
            format: None,
            target: None,
            jwt_token: None,
            oauth_issuer: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            graphql_introspection: true,
            graphql_depth_bypass: true,
            graphql_alias_overload: true,
            oauth_redirect: true,
            oauth_scope: true,
            oauth_state: true,
            oauth_grant: true,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
            common: CommonHttpArgs::default(),
        }
    }
}

/// Plain WAF testing configuration (no clap derives)
#[derive(Debug, Clone)]
pub struct WafConfig {
    pub url: String,
    pub detect_only: bool,
    pub bypass: bool,
    pub header_bypass: bool,
    pub smuggling: bool,
    pub evasion: bool,
    pub profile: String,
    pub test_type: Option<String>,
    pub concurrency: usize,
    pub timeout: u64,
    pub json: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub output: Option<String>,
    pub common: CommonHttpArgs,
}

/// Plain WAF stress test configuration (no clap derives)
#[derive(Debug, Clone)]
pub struct WafStressConfig {
    pub url: String,
    pub concurrency: usize,
    pub timeout: u64,
    pub json: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub output: Option<String>,
    pub common: CommonHttpArgs,
}

#[cfg(feature = "cli")]
impl From<super::super::cli::FuzzArgs> for FuzzConfig {
    fn from(args: super::super::cli::FuzzArgs) -> Self {
        Self {
            url: args.url,
            payload_type: args.payload_type,
            mode: match args.mode {
                super::super::cli::FuzzMode::Sequential => FuzzMode::Sequential,
                super::super::cli::FuzzMode::Burst => FuzzMode::Burst,
                super::super::cli::FuzzMode::Adaptive => FuzzMode::Adaptive,
            },
            mutate: args.mutate,
            mutation_count: args.mutation_count,
            grammar_fuzz: args.grammar_fuzz,
            grammar_type: args.grammar_type,
            adaptive_rate: args.adaptive_rate,
            session: args.session,
            diffing: args.diffing,
            capture_baseline: args.capture_baseline,
            enhanced_redos: args.enhanced_redos,
            waf_fingerprint: args.waf_fingerprint,
            chaining: args.chaining,
            chain_file: args.chain_file,
            method: args.method,
            param: args.param,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            output: args.output,
            verbose: args.verbose,
            quiet: args.quiet,
            format: args.format,
            target: args.target,
            jwt_token: args.jwt_token,
            oauth_issuer: args.oauth_issuer,
            oauth_client_id: args.oauth_client_id,
            oauth_client_secret: args.oauth_client_secret,
            idor_base_id: args.idor_base_id,
            idor_user_ids: args.idor_user_ids,
            ssti_param: args.ssti_param,
            graphql_introspection: args.graphql_introspection,
            graphql_depth_bypass: args.graphql_depth_bypass,
            graphql_alias_overload: args.graphql_alias_overload,
            oauth_redirect: args.oauth_redirect,
            oauth_scope: args.oauth_scope,
            oauth_state: args.oauth_state,
            oauth_grant: args.oauth_grant,
            schema: args.schema,
            discover_only: args.discover_only,
            auto_discover_schema: args.auto_discover_schema,
            calibrate: args.calibrate,
            fc: args.fc,
            fs: args.fs,
            fw: args.fw,
            fl: args.fl,
            ft: args.ft,
            fr: args.fr,
            common: args.common.into(),
        }
    }
}

#[cfg(feature = "cli")]
impl From<super::super::cli::WafStressArgs> for WafStressConfig {
    fn from(args: super::super::cli::WafStressArgs) -> Self {
        Self {
            url: args.url,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            verbose: args.verbose,
            quiet: args.quiet,
            output: args.output,
            common: args.common.into(),
        }
    }
}

impl From<WafStressConfig> for FuzzConfig {
    fn from(args: WafStressConfig) -> Self {
        FuzzConfig {
            url: args.url,
            payload_type: "all".to_string(),
            mode: FuzzMode::Sequential,
            mutate: false,
            mutation_count: 0,
            grammar_fuzz: false,
            grammar_type: None,
            adaptive_rate: false,
            session: false,
            diffing: false,
            capture_baseline: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            method: "GET".to_string(),
            param: None,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            output: args.output,
            verbose: args.verbose,
            quiet: args.quiet,
            format: None,
            target: None,
            jwt_token: None,
            oauth_issuer: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            oauth_redirect: false,
            oauth_scope: false,
            oauth_state: false,
            oauth_grant: false,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
            common: args.common,
        }
    }
}

#[cfg(feature = "cli")]
impl From<crate::cli::WafArgs> for WafConfig {
    fn from(args: crate::cli::WafArgs) -> Self {
        WafConfig {
            url: args.url,
            detect_only: args.detect_only,
            bypass: args.bypass,
            header_bypass: args.header_bypass,
            smuggling: args.smuggling,
            evasion: args.evasion,
            profile: args.profile,
            test_type: args.test_type,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            verbose: args.verbose,
            quiet: args.quiet,
            output: args.output,
            common: args.common.into(),
        }
    }
}
