use crate::error::Result;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::super::diff::ResponseDiffer;
use super::super::grammar::{Grammar, GrammarFuzzer};
use super::super::payloads::{get_all_payloads_cached, get_payloads, Payload, PayloadType};
use super::super::state::HttpSession;
use super::super::targets::get_target_payloads;

use crate::cli::{FuzzArgs, FuzzMode, WafStressArgs};
use crate::waf::types::Severity;

use super::super::detection::{PatternMatcher, TimingAnalyzer};
use super::types::{FuzzResult, FuzzSession};

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
    pub fn new(args: FuzzArgs) -> Result<Self> {
        Self::new_with_tui_mode(args, false)
    }

    pub fn new_with_tui_mode(args: FuzzArgs, tui_mode: bool) -> Result<Self> {
        let user_agent = args
            .common
            .user_agent
            .clone()
            .unwrap_or_else(crate::utils::stealth::tool_user_agent);

        let client = Self::build_client(&args)?;

        let grammar_fuzzer = if args.grammar_fuzz {
            let grammar = match args.grammar_type.as_deref() {
                Some("json") => Grammar::json(),
                Some("graphql") => Grammar::graphql(),
                Some("xml") => Grammar::xml(),
                Some("jwt") => Grammar::jwt(),
                Some("ssti") => Grammar::ssti(),
                _ => Grammar::json(),
            };
            Some(GrammarFuzzer::new(grammar))
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

    pub async fn run(&mut self) -> Result<()> {
        if self.args.verbose {
            eprintln!("Starting fuzz against {}", self.args.url);
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
            self.mutate_payloads(get_payloads(pt))
        } else {
            get_payloads(pt)
        };

        if self.args.grammar_fuzz {
            if let Some(ref mut grammar_fuzzer) = self.grammar_fuzzer {
                let grammar_payloads =
                    grammar_fuzzer.generate_batch(self.args.mutation_count.max(10));
                payloads.extend(grammar_payloads.into_iter().map(|p| super::super::payloads::Payload {
                    payload_type: PayloadType::Xss,
                    payload: p,
                    description: "Grammar-generated payload".to_string(),
                    severity: Severity::Medium,
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

    pub async fn run_all_types(&mut self) -> Result<()> {
        let payloads = if self.args.mutate {
            self.mutate_payloads(get_all_payloads_cached())
        } else {
            get_all_payloads_cached()
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
