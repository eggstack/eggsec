use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct ClientPool {
    clients: Arc<Vec<Client>>,
    current_index: Arc<AtomicUsize>,
}

impl ClientPool {
    pub fn new(
        pool_size: usize,
        timeout: Duration,
        insecure: bool,
        user_agent: Option<String>,
        proxy: Option<reqwest::Proxy>,
    ) -> Self {
        let user_agent = user_agent.unwrap_or_else(|| "Slapper/1.0".to_string());

        let mut builders = Vec::new();
        for _ in 0..pool_size {
            let mut builder = Client::builder()
                .timeout(timeout)
                .danger_accept_invalid_certs(insecure)
                .redirect(reqwest::redirect::Policy::limited(10))
                .pool_max_idle_per_host(pool_size / 2)
                .pool_idle_timeout(Duration::from_secs(30))
                .tcp_nodelay(true)
                .user_agent(user_agent.clone());

            if let Some(ref proxy) = proxy {
                builder = builder.proxy(proxy.clone());
            }

            builders.push(builder);
        }

        let clients: Vec<Client> = builders
            .into_iter()
            .filter_map(|b| b.build().ok())
            .collect();

        Self {
            clients: Arc::new(clients),
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn from_config(
        pool_size: usize,
        timeout: Duration,
        insecure: bool,
        user_agent: Option<String>,
        proxy: Option<reqwest::Proxy>,
    ) -> Self {
        Self::new(pool_size, timeout, insecure, user_agent, proxy)
    }

    pub fn get(&self) -> Client {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed) % self.clients.len();
        self.clients[index].clone()
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn pool_size(&self) -> usize {
        self.clients.len()
    }
}

impl Default for ClientPool {
    fn default() -> Self {
        Self::new(10, Duration::from_secs(30), false, None, None)
    }
}

pub struct OptimizedClientPool {
    pool: ClientPool,
}

impl OptimizedClientPool {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool: ClientPool::new(pool_size, Duration::from_secs(30), false, None, None),
        }
    }

    pub fn get(&self) -> Client {
        self.pool.get()
    }
}

impl Default for OptimizedClientPool {
    fn default() -> Self {
        Self::new(10)
    }
}
