use crate::cli::{GraphQlArgs, OAuthArgs, FuzzArgs, FuzzMode, CommonHttpArgs};
use anyhow::Result;

fn base_fuzz_args(url: String, concurrency: usize, timeout: u64, json: bool, output: Option<String>, verbose: bool, common: CommonHttpArgs) -> FuzzArgs {
    FuzzArgs {
        url,
        payload_type: String::new(),
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
        method: String::new(),
        param: None,
        concurrency,
        timeout,
        json,
        output,
        verbose,
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
        common,
    }
}

impl From<GraphQlArgs> for FuzzArgs {
    fn from(args: GraphQlArgs) -> Self {
        let mut fuzz_args = base_fuzz_args(
            args.url,
            args.concurrency,
            args.timeout,
            args.json,
            args.output,
            args.verbose,
            args.common,
        );
        fuzz_args.payload_type = "graphql".to_string();
        fuzz_args.method = "POST".to_string();
        fuzz_args
    }
}

impl From<OAuthArgs> for FuzzArgs {
    fn from(args: OAuthArgs) -> Self {
        let mut fuzz_args = base_fuzz_args(
            args.url,
            args.concurrency,
            args.timeout,
            args.json,
            args.output,
            args.verbose,
            args.common,
        );
        fuzz_args.payload_type = "oauth".to_string();
        fuzz_args.method = "GET".to_string();
        fuzz_args
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
