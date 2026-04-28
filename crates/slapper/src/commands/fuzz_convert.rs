use crate::cli::{GraphQlArgs, OAuthArgs, FuzzArgs, FuzzMode, CommonHttpArgs};
use anyhow::Result;

struct BaseFuzzConfig {
    url: String,
    concurrency: usize,
    timeout: u64,
    json: bool,
    output: Option<String>,
    verbose: bool,
    quiet: bool,
    common: CommonHttpArgs,
}

impl BaseFuzzConfig {
    fn into_fuzz_args(self, payload_type: String, method: String) -> FuzzArgs {
        FuzzArgs {
            url: self.url,
            payload_type,
            mode: FuzzMode::Sequential,
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
            method,
            param: None,
            concurrency: self.concurrency,
            timeout: self.timeout,
            json: self.json,
            output: self.output,
            verbose: self.verbose,
            quiet: self.quiet,
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
            common: self.common,
        }
    }
}

impl From<GraphQlArgs> for FuzzArgs {
    fn from(args: GraphQlArgs) -> Self {
        BaseFuzzConfig {
            url: args.url,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            output: args.output,
            verbose: args.verbose,
            quiet: args.quiet,
            common: args.common,
        }
        .into_fuzz_args("graphql".to_string(), "POST".to_string())
    }
}

impl From<OAuthArgs> for FuzzArgs {
    fn from(args: OAuthArgs) -> Self {
        BaseFuzzConfig {
            url: args.url,
            concurrency: args.concurrency,
            timeout: args.timeout,
            json: args.json,
            output: args.output,
            verbose: args.verbose,
            quiet: args.quiet,
            common: args.common,
        }
        .into_fuzz_args("oauth".to_string(), "GET".to_string())
    }
}

pub async fn run_graphql(args: GraphQlArgs) -> Result<()> {
    let fuzz_args = FuzzArgs::from(args);
    crate::fuzzer::run_cli(fuzz_args).await.map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn run_oauth(args: OAuthArgs) -> Result<()> {
    let fuzz_args = FuzzArgs::from(args);
    crate::fuzzer::run_cli(fuzz_args).await.map_err(|e| anyhow::anyhow!("{}", e))
}
