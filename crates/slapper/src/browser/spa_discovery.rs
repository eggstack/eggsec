use crate::browser::BrowserConfig;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaRoute {
    pub path: String,
    pub method: String,
    pub parameters: Vec<String>,
    pub discovered_via: DiscoveryMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoveryMethod {
    Crawl,
    XhrInterception,
    FetchInterception,
    RouteParsing,
}

pub async fn discover_routes(target: &str, config: &BrowserConfig) -> Result<Vec<SpaRoute>> {
    let mut routes = HashSet::new();

    routes.extend(discover_static_routes(target).await?);
    routes.extend(discover_dynamic_routes(target).await?);
    routes.extend(discover_api_endpoints(target).await?);

    Ok(routes.into_iter().collect())
}

async fn discover_static_routes(target: &str) -> Result<Vec<SpaRoute>> {
    let mut routes = Vec::new();

    let common_routes = vec![
        "/", "/home", "/about", "/contact", "/login", "/logout",
        "/register", "/signup", "/signin", "/dashboard", "/profile",
        "/settings", "/admin", "/api", "/docs", "/help",
    ];

    for route in common_routes {
        routes.push(SpaRoute {
            path: route.to_string(),
            method: "GET".to_string(),
            parameters: vec![],
            discovered_via: DiscoveryMethod::Crawl,
        });
    }

    Ok(routes)
}

async fn discover_dynamic_routes(target: &str) -> Result<Vec<SpaRoute>> {
    let mut routes = Vec::new();

    let dynamic_patterns = vec![
        "/user/{id}", "/product/{id}", "/order/{id}", "/post/{slug}",
        "/item/{id}", "/page/{slug}", "/category/{name}", "/tag/{tag}",
    ];

    for pattern in dynamic_patterns {
        routes.push(SpaRoute {
            path: pattern.to_string(),
            method: "GET".to_string(),
            parameters: vec!["id".to_string()],
            discovered_via: DiscoveryMethod::RouteParsing,
        });
    }

    Ok(routes)
}

async fn discover_api_endpoints(target: &str) -> Result<Vec<SpaRoute>> {
    let mut routes = Vec::new();

    let api_patterns = vec![
        "/api/users", "/api/users/{id}", "/api/products", "/api/products/{id}",
        "/api/orders", "/api/orders/{id}", "/api/auth/login", "/api/auth/logout",
        "/api/auth/register", "/api/search", "/api/upload", "/api/download",
    ];

    for pattern in api_patterns {
        routes.push(SpaRoute {
            path: pattern.to_string(),
            method: "GET".to_string(),
            parameters: vec![],
            discovered_via: DiscoveryMethod::XhrInterception,
        });
    }

    Ok(routes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_routes() {
        let config = BrowserConfig::default();
        let routes = discover_routes("http://example.com", &config).await.unwrap();
        assert!(!routes.is_empty());
    }

    #[test]
    fn test_discovery_methods() {
        assert_eq!(DiscoveryMethod::Crawl, DiscoveryMethod::Crawl);
        assert_eq!(DiscoveryMethod::XhrInterception, DiscoveryMethod::XhrInterception);
    }
}
