use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::constants;

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
        let user_agent = user_agent.unwrap_or_else(|| "Eggsec/1.0".to_string());

        let mut builders = Vec::new();
        for _ in 0..pool_size {
            let mut builder = Client::builder()
                .timeout(timeout)
                .danger_accept_invalid_certs(insecure)
                .redirect(reqwest::redirect::Policy::limited(
                    constants::http::DEFAULT_MAX_REDIRECTS as usize,
                ))
                .pool_max_idle_per_host(constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
                .pool_idle_timeout(Duration::from_secs(
                    constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
                ))
                .tcp_nodelay(true)
                .user_agent(user_agent.clone());

            if let Some(ref proxy) = proxy {
                builder = builder.proxy(proxy.clone());
            }

            builders.push(builder);
        }

        let mut clients = Vec::new();
        for builder in builders {
            match builder.build() {
                Ok(client) => clients.push(client),
                Err(e) => {
                    tracing::warn!("Failed to build HTTP client: {}", e);
                }
            }
        }

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

    pub fn get(&self) -> Option<Client> {
        if self.clients.is_empty() {
            return None;
        }
        let index = self.current_index.fetch_add(1, Ordering::Relaxed) % self.clients.len();
        Some(self.clients[index].clone())
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn pool_size(&self) -> usize {
        self.clients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}

impl Default for ClientPool {
    fn default() -> Self {
        Self::new(
            10,
            Duration::from_secs(constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS),
            false,
            None,
            None,
        )
    }
}

pub struct OptimizedClientPool {
    pool: ClientPool,
}

impl OptimizedClientPool {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool: ClientPool::new(
                pool_size,
                Duration::from_secs(constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS),
                false,
                None,
                None,
            ),
        }
    }

    pub fn get(&self) -> Option<Client> {
        self.pool.get()
    }
}

impl Default for OptimizedClientPool {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_pool_new_creates_clients() {
        let pool = ClientPool::new(3, Duration::from_secs(10), false, None, None);
        assert_eq!(pool.len(), 3);
        assert_eq!(pool.pool_size(), 3);
        assert!(!pool.is_empty());
    }

    #[test]
    fn test_client_pool_default() {
        let pool = ClientPool::default();
        assert_eq!(pool.len(), 10);
        assert!(!pool.is_empty());
    }

    #[test]
    fn test_client_pool_get_returns_client() {
        let pool = ClientPool::new(2, Duration::from_secs(5), false, None, None);
        let client = pool.get();
        assert!(client.is_some());
    }

    #[test]
    fn test_client_pool_round_robin() {
        let pool = ClientPool::new(2, Duration::from_secs(5), false, None, None);
        let client1 = pool.get();
        let client2 = pool.get();
        let client3 = pool.get();

        assert!(client1.is_some());
        assert!(client2.is_some());
        assert!(client3.is_some());
    }

    #[test]
    fn test_client_pool_from_config() {
        let pool = ClientPool::from_config(5, Duration::from_secs(15), true, None, None);
        assert_eq!(pool.len(), 5);
    }

    #[test]
    fn test_client_pool_custom_user_agent() {
        let pool = ClientPool::new(
            1,
            Duration::from_secs(5),
            false,
            Some("CustomAgent/1.0".to_string()),
            None,
        );
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_client_pool_insecure_mode() {
        let pool = ClientPool::new(1, Duration::from_secs(5), true, None, None);
        assert!(!pool.is_empty());
    }

    #[test]
    fn test_optimized_client_pool_new() {
        let pool = OptimizedClientPool::new(3);
        let client = pool.get();
        assert!(client.is_some());
    }

    #[test]
    fn test_optimized_client_pool_default() {
        let pool = OptimizedClientPool::default();
        let client = pool.get();
        assert!(client.is_some());
    }

    #[test]
    fn test_client_pool_single_client_reuse() {
        let pool = ClientPool::new(1, Duration::from_secs(5), false, None, None);
        let c1 = pool.get();
        let c2 = pool.get();
        assert!(c1.is_some());
        assert!(c2.is_some());
    }

    #[test]
    fn test_client_pool_len_matches_pool_size() {
        let pool = ClientPool::new(7, Duration::from_secs(10), false, None, None);
        assert_eq!(pool.len(), pool.pool_size());
    }
}
