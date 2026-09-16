//! Transport-neutral load-test executor (Phase D WS1/WS3).
//!
//! [`LoadTestExecutor`] issues [`LoadTestPlan`] requests through any
//! [`HttpTransport`] under a caller-supplied [`NetworkAuthority`]. It knows
//! nothing about Clap args, `EggsecConfig`, terminal progress, Reqwest
//! builders, or filesystem output.
//!
//! Concurrency: each worker owns a private [`Metrics`] accumulator; the run
//! merges them at the end, so completed requests never serialize on a shared
//! async mutex. Rate pacing uses per-worker spacing (no global semaphore
//! permit storm); cancellation terminates issuance and rate waiters promptly
//! via [`CancellationToken`] plus drop-cancellation of the worker futures.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eggsec_transport::{HttpTransport, NetworkAuthority};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use super::adapter::RequestTemplate;
use super::metrics::{LoadTestErrorKind, LoadTestResults, Metrics};
use super::plan::{LoadTestPlan, RatePolicy};
use super::progress::{LoadTestEvent, ProgressSink, SharedProgress};

/// Transport-neutral load-test executor.
pub struct LoadTestExecutor<T: HttpTransport> {
    plan: LoadTestPlan,
    template: RequestTemplate,
    transport: Arc<T>,
    authority: Arc<dyn NetworkAuthority>,
    cancellation: CancellationToken,
}

impl<T: HttpTransport + 'static> LoadTestExecutor<T> {
    /// Compose an executor. The caller supplies the scoped transport, the
    /// operation's authority, and a cancellation token; progress events go
    /// to `sink` (see [`ProgressSink`]).
    pub fn new(
        plan: LoadTestPlan,
        template: RequestTemplate,
        transport: Arc<T>,
        authority: Arc<dyn NetworkAuthority>,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            plan,
            template,
            transport,
            authority,
            cancellation,
        }
    }

    /// Run the plan to completion (or cancellation) and return merged results.
    pub async fn run<S: ProgressSink>(&self, sink: &S) -> Result<LoadTestResults, String>
    where
        T: 'static,
    {
        let worker_count = self.plan.worker_count();
        let issued = Arc::new(AtomicU64::new(0));
        let progress = SharedProgress::new();
        let total = self.plan.total_requests;
        let start = Instant::now();

        let mut workers = JoinSet::new();
        let pacer = GlobalPacer::new(self.plan.rate);
        for _ in 0..worker_count {
            let ctx = WorkerCtx {
                plan: self.plan.clone(),
                template: self.template.clone(),
                transport: self.transport.clone(),
                authority: self.authority.clone(),
                token: self.cancellation.clone(),
                issued: issued.clone(),
                progress: progress.clone(),
                pacer: pacer.clone(),
                total,
            };
            workers.spawn(async move { ctx.run().await });
        }

        let mut merged = Metrics::new(self.plan.url.clone());
        let mut worker_errors = 0u32;
        while let Some(join_result) = workers.join_next().await {
            match join_result {
                Ok(worker_metrics) => merged.merge(&worker_metrics),
                Err(e) if e.is_panic() => {
                    tracing::error!("Load test worker panicked: {:?}", e);
                    worker_errors = worker_errors.saturating_add(1);
                }
                Err(e) => {
                    tracing::error!("Load test worker failed: {}", e);
                    worker_errors = worker_errors.saturating_add(1);
                }
            }
            let completed = progress.get();
            sink.on_event(LoadTestEvent::RequestCompleted { completed, total });
        }
        if worker_errors > 0 {
            tracing::warn!("{worker_errors} loadtest worker(s) failed; totals are partial");
        }

        let completed = progress.get();
        sink.on_event(LoadTestEvent::Finished { completed, total });
        Ok(merged.to_results(start.elapsed()))
    }
}

/// Global issue pacer without a shared mutex.
///
/// Workers claim issue slots via a CAS loop on an atomic nanos counter, so
/// aggregate throughput matches the configured rate regardless of worker
/// count. No mutex is held across sleeps; cancellation preempts waits.
#[derive(Debug)]
struct GlobalPacer {
    interval_nanos: u64,
    start: Instant,
    next_nanos: AtomicU64,
}

impl GlobalPacer {
    fn new(rate: RatePolicy) -> Option<Arc<Self>> {
        let interval = rate.min_interval()?;
        let nanos = interval.as_nanos().min(u64::MAX as u128) as u64;
        if nanos == 0 {
            return None;
        }
        Some(Arc::new(Self {
            interval_nanos: nanos,
            start: Instant::now(),
            next_nanos: AtomicU64::new(0),
        }))
    }

    /// Claim the next issue slot, sleeping until it arrives.
    /// Returns `false` when cancelled before the slot.
    async fn wait(&self, token: &CancellationToken) -> bool {
        let slot = loop {
            let now_nanos = self.start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
            let next = self.next_nanos.load(Ordering::Relaxed);
            let slot = next.max(now_nanos);
            let new_next = slot.saturating_add(self.interval_nanos);
            match self.next_nanos.compare_exchange_weak(
                next,
                new_next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break slot,
                Err(_) => continue,
            }
        };
        let now_nanos = self.start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        if slot > now_nanos {
            let wait = Duration::from_nanos(slot - now_nanos);
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                () = token.cancelled() => return false,
            }
        }
        true
    }
}
/// Worker-owned execution context (keeps the spawned future under the
/// clippy argument-count lint while sharing issuance/progress state).
struct WorkerCtx<T: HttpTransport> {
    plan: LoadTestPlan,
    template: RequestTemplate,
    transport: Arc<T>,
    authority: Arc<dyn NetworkAuthority>,
    token: CancellationToken,
    issued: Arc<AtomicU64>,
    progress: Arc<SharedProgress>,
    pacer: Option<Arc<GlobalPacer>>,
    total: u64,
}

impl<T: HttpTransport> WorkerCtx<T> {
    /// One worker: pull indices atomically, pace, dispatch, record locally.
    async fn run(&self) -> Metrics {
        let mut metrics = Metrics::new(self.plan.url.clone());
        loop {
            if self.token.is_cancelled() {
                break;
            }
            let index = self.issued.fetch_add(1, Ordering::Relaxed);
            if index >= self.total {
                break;
            }
            if let Some(ref pacer) = self.pacer {
                if !pacer.wait(&self.token).await {
                    metrics.record_cancelled(Duration::from_millis(0));
                    self.progress.inc();
                    break;
                }
            }
            if self.token.is_cancelled() {
                metrics.record_cancelled(Duration::from_millis(0));
                self.progress.inc();
                break;
            }

            let request_start = Instant::now();
            let scoped = match self.template.scoped_request(&self.plan) {
                Ok(r) => r,
                Err(e) => {
                    metrics.record_transport_error(
                        LoadTestErrorKind::InvalidRequest,
                        format!("invalid request: {e}"),
                        request_start.elapsed(),
                    );
                    self.progress.inc();
                    continue;
                }
            };
            // Dropping the transport future on cancellation performs no further
            // hops (contract requirement); race it against the token so a
            // cancelled run doesn't wait out per-request timeouts.
            let dispatch = self.transport.execute(self.authority.as_ref(), scoped);
            tokio::select! {
                result = dispatch => {
                    match result {
                        Ok(response) => {
                            let latency = request_start.elapsed();
                            metrics.record_http_response(latency, response.status.as_u16());
                        }
                        Err(e) => {
                            let latency = request_start.elapsed();
                            let kind = LoadTestErrorKind::from_transport_error(&e);
                            metrics.record_transport_error(kind, e.to_string(), latency);
                        }
                    }
                    self.progress.inc();
                }
                () = self.token.cancelled() => {
                    metrics.record_cancelled(request_start.elapsed());
                    self.progress.inc();
                    break;
                }
            }
        }
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loadtest::progress::NoopSink;
    use eggsec_transport::{
        CannedResponse, InMemoryResolver, PolicyCheckpoint, RecordingFakeTransport, TransportError,
    };
    use std::net::IpAddr;
    use url::Url;

    #[derive(Debug)]
    struct AllowAll;

    impl NetworkAuthority for AllowAll {
        fn authorize_initial_url(&self, url: &Url) -> Result<(), TransportError> {
            eggsec_transport::reject_url_userinfo(url)
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Ok(c.to_vec())
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Ok(())
        }
        fn authorize_redirect(&self, _f: &Url, t: &Url) -> Result<(), TransportError> {
            self.authorize_initial_url(t)
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Ok(())
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct DenyAll;

    impl NetworkAuthority for DenyAll {
        fn authorize_initial_url(&self, _u: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::InitialUrl, "deny"))
        }
        fn authorize_host(
            &self,
            _h: &str,
            _p: Option<u16>,
            _l: bool,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Host, "deny"))
        }
        fn authorize_resolved(
            &self,
            _h: &str,
            _c: &[IpAddr],
        ) -> Result<Vec<IpAddr>, TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Dns, "deny"))
        }
        fn authorize_socket(&self, _h: &str, _a: IpAddr, _p: u16) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Socket, "deny"))
        }
        fn authorize_redirect(&self, _f: &Url, _t: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Redirect, "deny"))
        }
        fn authorize_proxy(&self, _p: &Url, _u: &Url) -> Result<(), TransportError> {
            Err(TransportError::denied(PolicyCheckpoint::Proxy, "deny"))
        }
        fn check_tls_consistency(
            &self,
            _h: &str,
            _s: Option<&str>,
            _o: Option<&str>,
        ) -> Result<(), TransportError> {
            Err(TransportError::denied(
                PolicyCheckpoint::TlsConsistency,
                "deny",
            ))
        }
    }

    fn fake_with(host: &str, status: eggsec_transport::StatusCode) -> Arc<RecordingFakeTransport> {
        let resolver = Arc::new(InMemoryResolver::new().with(host, vec!["93.184.216.34"]));
        Arc::new(RecordingFakeTransport::new(resolver).with_canned(
            &format!("http://{host}/"),
            CannedResponse {
                status,
                location: None,
                body: b"ok".to_vec(),
            },
        ))
    }

    fn plan_for(url: &str, n: u64, c: usize) -> (LoadTestPlan, RequestTemplate) {
        let plan = LoadTestPlan::new(url.to_string(), n, c, Duration::from_secs(5)).expect("plan");
        let template = RequestTemplate {
            headers: Vec::new(),
            body: None,
            user_agent: "test".to_string(),
            timeout: eggsec_transport::TimeoutPolicy::with_request_timeout(5),
            redirect: eggsec_transport::RedirectPolicy::SameHostOnly { max_redirects: 5 },
            proxy: eggsec_transport::ProxyIntent::Direct,
            tls: eggsec_transport::TlsPolicy::verified(),
        };
        (plan, template)
    }

    #[tokio::test]
    async fn executor_counts_success_through_fake() {
        let transport = fake_with("example.com", eggsec_transport::StatusCode::OK);
        let (plan, template) = plan_for("http://example.com/", 20, 4);
        let ex = LoadTestExecutor::new(
            plan,
            template,
            transport.clone(),
            Arc::new(AllowAll),
            CancellationToken::new(),
        );
        let results = ex.run(&NoopSink).await.expect("run");
        assert_eq!(results.total_requests, 20);
        assert_eq!(results.successful_requests, 20);
        assert_eq!(transport.hop_count(), 20);
    }

    #[tokio::test]
    async fn executor_denials_are_categorized_not_exposed() {
        let transport = fake_with("example.com", eggsec_transport::StatusCode::OK);
        let (plan, template) = plan_for("http://example.com/", 5, 2);
        let ex = LoadTestExecutor::new(
            plan,
            template,
            transport,
            Arc::new(DenyAll),
            CancellationToken::new(),
        );
        let results = ex.run(&NoopSink).await.expect("run");
        assert_eq!(results.total_requests, 5);
        assert_eq!(results.failed_requests, 5);
        assert_eq!(results.error_kinds.get("policy_denied"), Some(&5));
        for err in &results.errors {
            assert!(
                !err.contains("RecordingFakeTransport"),
                "backend leak: {err}"
            );
        }
    }

    #[tokio::test]
    async fn executor_cancellation_yields_partial_totals() {
        let transport = fake_with("example.com", eggsec_transport::StatusCode::OK);
        let (plan, template) = plan_for("http://example.com/", 10_000, 4);
        let token = CancellationToken::new();
        let ex =
            LoadTestExecutor::new(plan, template, transport, Arc::new(AllowAll), token.clone());
        token.cancel();
        let results = ex.run(&NoopSink).await.expect("run");
        assert!(results.total_requests <= 10_000);
        assert_eq!(
            results.total_requests,
            results.successful_requests + results.failed_requests
        );
    }

    #[tokio::test]
    async fn executor_emits_structured_progress() {
        use std::sync::Mutex;
        let transport = fake_with("example.com", eggsec_transport::StatusCode::OK);
        let (plan, template) = plan_for("http://example.com/", 10, 2);
        let ex = LoadTestExecutor::new(
            plan,
            template,
            transport,
            Arc::new(AllowAll),
            CancellationToken::new(),
        );
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen2 = seen.clone();
        let sink = super::super::progress::FnSink(move |e| {
            seen2.lock().expect("lock").push(e);
        });
        let results = ex.run(&sink).await.expect("run");
        assert_eq!(results.total_requests, 10);
        let events = seen.lock().expect("lock");
        assert!(!events.is_empty());
        assert!(matches!(
            events.last().expect("event"),
            LoadTestEvent::Finished {
                completed: 10,
                total: 10
            }
        ));
    }
}
