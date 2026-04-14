use crate::error::Result;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::super::diff::ResponseDiffer;
use super::super::grammar::{Grammar, GrammarFuzzer, GrammarKind};
use super::super::payloads::{get_all_payloads_cached, get_payloads, Payload, PayloadType};
use super::super::state::HttpSession;
use super::super::targets::get_target_payloads;

use crate::cli::{FuzzArgs, FuzzMode, WafStressArgs};
use crate::utils::sanitize_for_logging;
use crate::waf::types::Severity;

use super::super::detection::{PatternMatcher, TimingAnalyzer};
use super::types::{FuzzResult, FuzzSession};

/// The main fuzzing engine that orchestrates payload generation, HTTP request
/// execution, and vulnerability detection.
///
/// `FuzzEngine` supports multiple fuzzing modes:
/// - **Payload-based**: Uses predefined payloads from the payload library
/// - **Grammar-based**: Generates payloads from formal grammars (JSON, GraphQL, XML, JWT, SSTI)
/// - **Mutation-based**: Mutates existing payloads to discover edge cases
///
/// # Examples
///
/// ```no_run
/// use slapper::cli::{FuzzArgs, CommonHttpArgs};
/// use slapper::fuzzer::engine::FuzzEngine;
///
/// # fn main() -> slapper::error::Result<()> {
/// let args = FuzzArgs {
///     target: "http://example.com".to_string(),
///     payload_type: "sqli".to_string(),
///     concurrency: 10,
///     timeout: 30,
///     // ... other fields
///     ..Default::default()
/// };
/// let engine = FuzzEngine::new(args)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns [`SlapperError`](crate::error::SlapperError) if the HTTP client
/// cannot be built (e.g., invalid proxy configuration).
pub struct FuzzEngine {
    pub(crate) args: FuzzArgs,
    pub(crate) client: Client,
    pub(crate) timing_analyzer: Arc<Mutex<TimingAnalyzer>>,
    pub(crate) pattern_matcher: PatternMatcher,
    pub(crate) user_agent: String,
    pub(crate) tui_mode: bool,
    pub(crate) grammar_fuzzer: Option<GrammarFuzzer>,
    pub(crate) http_session: Option<HttpSession>,
    pub(crate) differ: Option<ResponseDiffer>,
    pub(crate) baseline_captured: bool,
    #[cfg(feature = "ai-integration")]
    pub(crate) ai_generator: Option<crate::ai::AiPayloadGenerator>,
}

impl FuzzEngine {
    /// Creates a new `FuzzEngine` with the given arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn new(args: FuzzArgs) -> Result<Self> {
        Self::new_with_tui_mode(args, false)
    }

    /// Creates a new `FuzzEngine` with explicit TUI mode control.
    ///
    /// When `tui_mode` is true, progress indicators use the TUI-compatible
    /// format instead of terminal progress bars.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn new_with_tui_mode(args: FuzzArgs, tui_mode: bool) -> Result<Self> {
        let user_agent = args
            .common
            .user_agent
            .clone()
            .unwrap_or_else(crate::utils::stealth::tool_user_agent);

        let client = Self::build_client(&args)?;

        let grammar_fuzzer = if args.grammar_fuzz {
            let (grammar, kind) = match args.grammar_type.as_deref() {
                Some("json") => (Grammar::json(), GrammarKind::Json),
                Some("graphql") => (Grammar::graphql(), GrammarKind::GraphQL),
                Some("xml") => (Grammar::xml(), GrammarKind::Xml),
                Some("jwt") => (Grammar::jwt(), GrammarKind::Jwt),
                Some("ssti") => (Grammar::ssti(), GrammarKind::Ssti),
                _ => (Grammar::json(), GrammarKind::Json),
            };
            Some(GrammarFuzzer::new(grammar, kind))
        } else {
            None
        };

        let http_session = if args.session {
            Some(HttpSession::new())
        } else {
            None
        };

        let differ = if args.diffing {
            Some(ResponseDiffer::new())
        } else {
            None
        };

        Ok(Self {
            args,
            client,
            timing_analyzer: Arc::new(Mutex::new(TimingAnalyzer::new())),
            pattern_matcher: PatternMatcher::new(),
            user_agent,
            tui_mode,
            grammar_fuzzer,
            http_session,
            differ,
            baseline_captured: false,
            #[cfg(feature = "ai-integration")]
            ai_generator: None,
        })
    }

    fn build_client(args: &FuzzArgs) -> Result<Client> {
        let concurrency = args.concurrency.clamp(1, 500);

        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(args.timeout))
            .danger_accept_invalid_certs(args.common.insecure)
            .redirect(reqwest::redirect::Policy::limited(10))
            .pool_max_idle_per_host(concurrency / 2)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_nodelay(true);

        if let Some(proxy_url) = &args.common.proxy {
            if let Ok(mut proxy) = reqwest::Proxy::all(proxy_url) {
                if let Some(auth) = &args.common.proxy_auth {
                    let parts: Vec<&str> = auth.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        proxy = proxy.basic_auth(parts[0], parts[1]);
                    }
                }
                client_builder = client_builder.proxy(proxy);
            }
        }

        client_builder.build().map_err(|e| {
            crate::error::SlapperError::from(e).with_timeout(args.timeout * 1000)
        })
    }

    pub fn new_from_waf_args(args: WafStressArgs) -> Result<Self> {
        Self::new(FuzzArgs::from(args))
    }

    #[cfg(feature = "ai-integration")]
    pub fn set_ai_generator(&mut self, generator: crate::ai::AiPayloadGenerator) {
        self.ai_generator = Some(generator);
    }

    #[cfg(feature = "ai-integration")]
    pub fn ai_generator(&self) -> Option<&crate::ai::AiPayloadGenerator> {
        self.ai_generator.as_ref()
    }

    /// Executes the fuzzing session and prints results.
    ///
    /// This is the main entry point for CLI-based fuzzing. It runs the
    /// configured payload types against the target URL and outputs results
    /// in the requested format (JSON, pretty-printed, or compact).
    ///
    /// # Errors
    ///
    /// Returns an error if HTTP requests fail or results cannot be serialized.
    pub async fn run(&mut self) -> Result<()> {
        if self.args.verbose {
            eprintln!("Starting fuzz against {}", sanitize_for_logging(&self.args.url));
        }

        let session = self.run_return_session().await?;

        let output = if self.args.json {
            serde_json::to_string_pretty(&session)?
        } else {
            session.to_string()
        };

        if let Some(ref output_file) = self.args.output {
            tokio::fs::write(output_file, &output).await?;
            if self.args.verbose {
                eprintln!("Results written to {}", output_file);
            }
        } else {
            println!("{}", output);
        }

        if self.args.verbose {
            eprintln!(
                "Fuzz complete: {} requests, {} findings",
                session.total_requests, session.findings
            );
        }

        Ok(())
    }

    async fn prepare_payloads(&mut self, pt: PayloadType) -> Result<Vec<Payload>> {
        let mut payloads = if self.args.mutate {
            self.mutate_payloads(&get_payloads(pt))
        } else {
            get_payloads(pt)
        };

        if self.args.grammar_fuzz {
            if let Some(ref mut grammar_fuzzer) = self.grammar_fuzzer {
                let grammar_payloads =
                    grammar_fuzzer.generate_batch(self.args.mutation_count.max(10));
                let payload_type = grammar_fuzzer.kind().payload_type();
                payloads.extend(grammar_payloads.into_iter().map(|p| super::super::payloads::Payload {
                    payload_type,
                    payload: p,
                    description: "Grammar-generated payload".to_string(),
                    severity: grammar_fuzzer.kind().severity(),
                    tags: vec!["grammar".to_string()],
                }));
            }
        }

        #[cfg(feature = "ai-integration")]
        if let Some(ref ai_gen) = self.ai_generator {
            let vuln_type = format!("{:?}", pt);
            let context = format!("target={}", self.args.url);
            if let Ok(ai_payloads) = ai_gen.generate_payloads(&vuln_type, &context).await {
                payloads.extend(ai_payloads.into_iter().map(|p| super::super::payloads::Payload {
                    payload_type: pt,
                    payload: p,
                    description: "AI-generated payload".to_string(),
                    severity: Severity::Medium,
                    tags: vec!["ai-generated".to_string()],
                }));
            } else {
                tracing::warn!("AI payload generation failed for {} (context: {})", vuln_type, context);
            }
        }

        Ok(payloads)
    }

    async fn run_payload_batch(&mut self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
        let results = match self.args.mode {
            FuzzMode::Sequential => self.run_sequential_with_session(payloads).await?,
            FuzzMode::Burst => self.run_burst_with_session(payloads).await?,
            FuzzMode::Adaptive => self.run_adaptive_with_session(payloads).await?,
        };

        if self.args.diffing && self.differ.is_some() {
            self.apply_diffing(results).await
        } else {
            Ok(results)
        }
    }

    pub async fn run_return_session(&mut self) -> Result<FuzzSession> {
        if self.args.capture_baseline && self.differ.is_some() {
            self.capture_baseline_for_diffing().await?;
            self.baseline_captured = true;
        }

        let payload_types = self.parse_payload_types()?;
        let mut all_results = Vec::with_capacity(2048);
        let start = Instant::now();

        for pt in payload_types {
            if pt.is_advanced() {
                let pt_str = format!("{:?}", pt).to_lowercase();
                all_results.extend(self.run_advanced_fuzzer(&pt_str).await?);
            } else {
                let payloads = self.prepare_payloads(pt).await?;
                all_results.extend(self.run_payload_batch(payloads).await?);
            }
        }

        if let Some(ref target_str) = self.args.target {
            if let Ok(target_type) = target_str.parse::<super::super::targets::TargetType>() {
                let target_payloads = get_target_payloads(target_type);
                let payloads: Vec<super::super::payloads::Payload> = target_payloads
                    .into_iter()
                    .map(|tp| super::super::payloads::Payload {
                        payload_type: PayloadType::Traversal,
                        payload: tp.payload,
                        description: tp.description,
                        severity: Severity::High,
                        tags: vec![target_type.to_string(), tp.category],
                    })
                    .collect();

                let results = match self.args.mode {
                    FuzzMode::Sequential => self.run_sequential(payloads).await?,
                    FuzzMode::Burst => self.run_burst(payloads).await?,
                    FuzzMode::Adaptive => self.run_adaptive(payloads).await?,
                };
                all_results.extend(results);
            }
        }

        Ok(self.build_session(all_results, start.elapsed(), None))
    }

    /// Runs fuzzing against all payload types sequentially.
    ///
    /// This method iterates through every available [`PayloadType`] and
    /// sends all payloads for each type to the target. Use this for
    /// comprehensive coverage when you don't know which vulnerability
    /// class might be present.
    ///
    /// # Errors
    ///
    /// Returns an error if any HTTP request fails critically.
    pub async fn run_all_types(&mut self) -> Result<()> {
        let payloads: Vec<Payload> = if self.args.mutate {
            self.mutate_payloads(get_all_payloads_cached())
        } else {
            get_all_payloads_cached().clone()
        };

        let start = Instant::now();
        let results = match self.args.mode {
            FuzzMode::Sequential => self.run_sequential(payloads).await?,
            FuzzMode::Burst => self.run_burst(payloads).await?,
            FuzzMode::Adaptive => self.run_adaptive(payloads).await?,
        };

        let session = self.build_session(results, start.elapsed(), None);

        if self.args.json {
            println!("{}", serde_json::to_string_pretty(&session)?);
        } else {
            println!("{}", session);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CommonHttpArgs, FuzzMode};

    fn make_fuzz_args(url: &str) -> FuzzArgs {
        FuzzArgs {
            url: url.to_string(),
            payload_type: "sqli".to_string(),
            common: CommonHttpArgs::default(),
            method: "GET".to_string(),
            param: None,
            concurrency: 10,
            timeout: 5,
            verbose: false,
            quiet: false,
            json: false,
            output: None,
            mutate: false,
            mutation_count: 5,
            grammar_fuzz: false,
            grammar_type: None,
            session: false,
            diffing: false,
            capture_baseline: false,
            mode: FuzzMode::Sequential,
            target: None,
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            jwt_token: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_redirect: false,
            oauth_scope: false,
            oauth_state: false,
            oauth_grant: false,
            oauth_issuer: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            adaptive_rate: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            format: None,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
        }
    }

    #[test]
    fn test_fuzz_engine_new() {
        let args = make_fuzz_args("http://example.com");
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert_eq!(engine.args.url, "http://example.com");
        assert_eq!(engine.tui_mode, false);
        assert!(engine.grammar_fuzzer.is_none());
        assert!(engine.http_session.is_none());
        assert!(engine.differ.is_none());
        assert!(!engine.baseline_captured);
    }

    #[test]
    fn test_fuzz_engine_new_with_tui_mode() {
        let args = make_fuzz_args("http://example.com");
        let engine = FuzzEngine::new_with_tui_mode(args, true);
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert_eq!(engine.tui_mode, true);
    }

    #[test]
    fn test_fuzz_engine_with_grammar_fuzz() {
        let mut args = make_fuzz_args("http://example.com");
        args.grammar_fuzz = true;
        args.grammar_type = Some("json".to_string());
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert!(engine.grammar_fuzzer.is_some());
    }

    #[test]
    fn test_fuzz_engine_with_session() {
        let mut args = make_fuzz_args("http://example.com");
        args.session = true;
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert!(engine.http_session.is_some());
    }

    #[test]
    fn test_fuzz_engine_with_diffing() {
        let mut args = make_fuzz_args("http://example.com");
        args.diffing = true;
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert!(engine.differ.is_some());
    }

    #[test]
    fn test_fuzz_engine_concurrency_clamped() {
        let mut args = make_fuzz_args("http://example.com");
        args.concurrency = 1000;
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_fuzz_engine_concurrency_minimum() {
        let mut args = make_fuzz_args("http://example.com");
        args.concurrency = 0;
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_fuzz_engine_user_agent_default() {
        let args = make_fuzz_args("http://example.com");
        let engine = FuzzEngine::new(args).unwrap();
        assert!(!engine.user_agent.is_empty());
    }

    #[test]
    fn test_fuzz_engine_user_agent_custom() {
        let mut args = make_fuzz_args("http://example.com");
        args.common.user_agent = Some("CustomAgent/1.0".to_string());
        let engine = FuzzEngine::new(args).unwrap();
        assert_eq!(engine.user_agent, "CustomAgent/1.0");
    }

    #[test]
    fn test_fuzz_engine_from_waf_args() {
        use crate::cli::WafStressArgs;
        let waf_args = WafStressArgs {
            url: "http://example.com".to_string(),
            concurrency: 20,
            timeout: 10,
            json: false,
            verbose: false,
            quiet: false,
            output: None,
            common: CommonHttpArgs::default(),
        };
        let engine = FuzzEngine::new_from_waf_args(waf_args);
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert_eq!(engine.args.url, "http://example.com");
        assert_eq!(engine.args.concurrency, 20);
    }

    #[tokio::test]
    async fn test_fuzz_engine_build_client_with_invalid_proxy() {
        let mut args = make_fuzz_args("http://example.com");
        args.common.proxy = Some("not-a-valid-proxy".to_string());
        let engine = FuzzEngine::new(args);
        assert!(engine.is_ok());
    }
}
