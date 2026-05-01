#[derive(Clone, Default)]
pub struct GlobalHttpOptions {
    pub insecure: bool,
    pub proxy: Option<String>,
    pub proxy_auth: Option<String>,
    pub auth: Option<String>,
    pub bearer: Option<String>,
    pub cookie: Option<String>,
    pub api_key: Option<String>,
    pub user_agent: Option<String>,
    pub stealth: bool,
    pub rate_limit: Option<u32>,
    pub jitter: Option<String>,
}
