use crate::error::Result;
use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;

use crate::utils::create_insecure_client_with_options;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TechStack {
    pub servers: Vec<String>,
    pub frameworks: Vec<String>,
    pub languages: Vec<String>,
    pub databases: Vec<String>,
    pub cdns: Vec<String>,
    pub cms: Vec<String>,
    pub javascript: Vec<String>,
    pub other: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechDetectionResult {
    pub url: String,
    pub status_code: u16,
    pub headers: FxHashMap<String, String>,
    pub tech_stack: TechStack,
}

pub struct TechDetector {
    client: reqwest::Client,
}

struct LowercaseHeaders {
    server: Option<String>,
    powered_by: Option<String>,
}

impl TechDetector {
    fn lowercase_headers(headers: &FxHashMap<String, String>) -> LowercaseHeaders {
        LowercaseHeaders {
            server: headers.get("server").map(|s| s.to_lowercase()),
            powered_by: headers.get("x-powered-by").map(|s| s.to_lowercase()),
        }
    }
}

impl TechDetector {
    pub fn new() -> Result<Self> {
        let client = create_insecure_client_with_options(15, |builder| {
            builder
                .redirect(reqwest::redirect::Policy::limited(5))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        })?;

        Ok(Self { client })
    }

    pub async fn detect(&self, url: &str) -> Result<TechDetectionResult> {
        let response = self.client.get(url).send().await?;
        let status = response.status().as_u16();

        let headers: FxHashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
            .collect();

        let body = response.text().await.unwrap_or_default();
        let body_lower = body.to_lowercase();

        let mut tech_stack = TechStack::default();
        let lowercase_headers = Self::lowercase_headers(&headers);

        self.detect_servers(lowercase_headers.server.as_deref(), &mut tech_stack);
        self.detect_frameworks(
            lowercase_headers.powered_by.as_deref(),
            &headers,
            &body_lower,
            &mut tech_stack,
        );
        self.detect_cms(lowercase_headers.powered_by.as_deref(), &mut tech_stack);
        self.detect_cdns(&headers, &mut tech_stack);
        self.detect_databases(lowercase_headers.server.as_deref(), &mut tech_stack);
        self.detect_javascript(&body_lower, &mut tech_stack);
        self.detect_languages(lowercase_headers.powered_by.as_deref(), &mut tech_stack);

        Ok(TechDetectionResult {
            url: url.to_string(),
            status_code: status,
            headers,
            tech_stack,
        })
    }

    fn detect_servers(&self, server_lower: Option<&str>, stack: &mut TechStack) {
        if let Some(s) = server_lower {
            if s.contains("nginx") && !stack.servers.contains(&"Nginx".to_string()) {
                stack.servers.push("Nginx".to_string());
            }
            if s.contains("apache") && !stack.servers.contains(&"Apache".to_string()) {
                stack.servers.push("Apache".to_string());
            }
            if (s.contains("microsoft-iis") || s.contains("iis"))
                && !stack.servers.contains(&"IIS".to_string()) {
                    stack.servers.push("IIS".to_string());
                }
            if s.contains("cloudflare") && !stack.cdns.contains(&"Cloudflare".to_string()) {
                stack.cdns.push("Cloudflare".to_string());
            }
            if s.contains("akamai") && !stack.cdns.contains(&"Akamai".to_string()) {
                stack.cdns.push("Akamai".to_string());
            }
            if s.contains("cloudfront") && !stack.cdns.contains(&"CloudFront".to_string()) {
                stack.cdns.push("CloudFront".to_string());
            }
            if s.contains("fastly") && !stack.cdns.contains(&"Fastly".to_string()) {
                stack.cdns.push("Fastly".to_string());
            }
            if (s.contains("lite-speed") || s.contains("litespeed"))
                && !stack.servers.contains(&"LiteSpeed".to_string()) {
                    stack.servers.push("LiteSpeed".to_string());
                }
            if s.contains("openresty")
                && !stack.servers.contains(&"OpenResty".to_string()) {
                    stack.servers.push("OpenResty".to_string());
                }
            if s.contains("caddy")
                && !stack.servers.contains(&"Caddy".to_string()) {
                    stack.servers.push("Caddy".to_string());
                }
            if s.contains("traefik") && !stack.frameworks.contains(&"Traefik".to_string()) {
                stack.frameworks.push("Traefik".to_string());
            }
        }
    }

    fn detect_frameworks(
        &self,
        powered_by_lower: Option<&str>,
        headers: &FxHashMap<String, String>,
        body: &str,
        stack: &mut TechStack,
    ) {
        let framework = headers.get("x-framework").map(|s| s.to_lowercase());

        if let Some(pb) = powered_by_lower {
            if pb.contains("express") && !stack.frameworks.contains(&"Express".to_string()) {
                stack.frameworks.push("Express".to_string());
            }
            if pb.contains("django") && !stack.frameworks.contains(&"Django".to_string()) {
                stack.frameworks.push("Django".to_string());
            }
            if (pb.contains("rails") || pb.contains("ruby on rails"))
                && !stack.frameworks.contains(&"Ruby on Rails".to_string()) {
                    stack.frameworks.push("Ruby on Rails".to_string());
                }
            if pb.contains("laravel") && !stack.frameworks.contains(&"Laravel".to_string()) {
                stack.frameworks.push("Laravel".to_string());
            }
            if pb.contains("spring") && !stack.frameworks.contains(&"Spring".to_string()) {
                stack.frameworks.push("Spring".to_string());
            }
            if pb.contains("asp.net") && !stack.frameworks.contains(&"ASP.NET".to_string()) {
                stack.frameworks.push("ASP.NET".to_string());
            }
            if pb.contains("cake") && !stack.frameworks.contains(&"CakePHP".to_string()) {
                stack.frameworks.push("CakePHP".to_string());
            }
            if pb.contains("codeigniter") && !stack.frameworks.contains(&"CodeIgniter".to_string())
            {
                stack.frameworks.push("CodeIgniter".to_string());
            }
            if pb.contains("symfony") && !stack.frameworks.contains(&"Symfony".to_string()) {
                stack.frameworks.push("Symfony".to_string());
            }
            if pb.contains("flask") && !stack.frameworks.contains(&"Flask".to_string()) {
                stack.frameworks.push("Flask".to_string());
            }
            if pb.contains("fastapi") && !stack.frameworks.contains(&"FastAPI".to_string()) {
                stack.frameworks.push("FastAPI".to_string());
            }
            if (pb.contains("next.js") || pb.contains("nextjs"))
                && !stack.frameworks.contains(&"Next.js".to_string()) {
                    stack.frameworks.push("Next.js".to_string());
                }
            if pb.contains("nuxt") && !stack.frameworks.contains(&"Nuxt.js".to_string()) {
                stack.frameworks.push("Nuxt.js".to_string());
            }
            if pb.contains("gatsby") && !stack.frameworks.contains(&"Gatsby".to_string()) {
                stack.frameworks.push("Gatsby".to_string());
            }
            if pb.contains("hugo") && !stack.frameworks.contains(&"Hugo".to_string()) {
                stack.frameworks.push("Hugo".to_string());
            }
            if pb.contains("jekyll") && !stack.frameworks.contains(&"Jekyll".to_string()) {
                stack.frameworks.push("Jekyll".to_string());
            }
        }

        if let Some(fw) = framework {
            if fw.contains("express") && !stack.frameworks.contains(&"Express".to_string()) {
                stack.frameworks.push("Express".to_string());
            }
            if fw.contains("django") && !stack.frameworks.contains(&"Django".to_string()) {
                stack.frameworks.push("Django".to_string());
            }
        }

        if (body.contains("wp-content") || body.contains("wp-includes"))
            && !stack.cms.contains(&"WordPress".to_string()) {
                stack.cms.push("WordPress".to_string());
            }
        if (body.contains("drupal") || body.contains("Drupal"))
            && !stack.cms.contains(&"Drupal".to_string()) {
                stack.cms.push("Drupal".to_string());
            }
        if (body.contains("joomla") || body.contains("Joomla"))
            && !stack.cms.contains(&"Joomla".to_string()) {
                stack.cms.push("Joomla".to_string());
            }
        if (body.contains("magento") || body.contains("Magento"))
            && !stack.cms.contains(&"Magento".to_string()) {
                stack.cms.push("Magento".to_string());
            }
        if (body.contains("shopify") || body.contains("Shopify"))
            && !stack.cms.contains(&"Shopify".to_string()) {
                stack.cms.push("Shopify".to_string());
            }
        if (body.contains("wp-json") || body.contains("wordpress"))
            && !stack.cms.contains(&"WordPress".to_string()) {
                stack.cms.push("WordPress".to_string());
            }

        if (body.contains("__vue") || body.contains("vue.js"))
            && !stack.javascript.contains(&"Vue.js".to_string()) {
                stack.javascript.push("Vue.js".to_string());
            }
        if body.contains("react") && body.contains("node_modules")
            && !stack.javascript.contains(&"React".to_string()) {
                stack.javascript.push("React".to_string());
            }
        if body.contains("angular") && body.contains("ng-")
            && !stack.javascript.contains(&"Angular".to_string()) {
                stack.javascript.push("Angular".to_string());
            }
        if body.contains("svelte")
            && !stack.javascript.contains(&"Svelte".to_string()) {
                stack.javascript.push("Svelte".to_string());
            }
    }

    fn detect_cms(&self, powered_by_lower: Option<&str>, stack: &mut TechStack) {
        if let Some(pb) = powered_by_lower {
            if pb.contains("wordpress") && !stack.cms.contains(&"WordPress".to_string()) {
                stack.cms.push("WordPress".to_string());
            }
            if pb.contains("drupal") && !stack.cms.contains(&"Drupal".to_string()) {
                stack.cms.push("Drupal".to_string());
            }
            if pb.contains("joomla") && !stack.cms.contains(&"Joomla".to_string()) {
                stack.cms.push("Joomla".to_string());
            }
        }
    }

    fn detect_cdns(&self, headers: &FxHashMap<String, String>, stack: &mut TechStack) {
        for (key, value) in headers {
            let key_lower = key.to_lowercase();
            let value_lower = value.to_lowercase();

            if (key_lower.contains("cf-ray") || value_lower.contains("cloudflare"))
                && !stack.cdns.contains(&"Cloudflare".to_string()) {
                    stack.cdns.push("Cloudflare".to_string());
                }
            if (key_lower.contains("akamai") || value_lower.contains("akamai"))
                && !stack.cdns.contains(&"Akamai".to_string()) {
                    stack.cdns.push("Akamai".to_string());
                }
            if (key_lower.contains("fastly") || value_lower.contains("fastly"))
                && !stack.cdns.contains(&"Fastly".to_string()) {
                    stack.cdns.push("Fastly".to_string());
                }
            if (key_lower.contains("cloudfront") || value_lower.contains("cloudfront"))
                && !stack.cdns.contains(&"CloudFront".to_string()) {
                    stack.cdns.push("CloudFront".to_string());
                }
            if (key_lower.contains("bunny") || value_lower.contains("bunny"))
                && !stack.cdns.contains(&"BunnyCDN".to_string()) {
                    stack.cdns.push("BunnyCDN".to_string());
                }
            if (key_lower.contains("keycdn") || value_lower.contains("keycdn"))
                && !stack.cdns.contains(&"KeyCDN".to_string()) {
                    stack.cdns.push("KeyCDN".to_string());
                }
            if (key_lower.contains("cdnjs") || value_lower.contains("cdnjs"))
                && !stack.cdns.contains(&"cdnjs".to_string()) {
                    stack.cdns.push("cdnjs".to_string());
                }
            if (key_lower.contains("unpkg") || value_lower.contains("unpkg"))
                && !stack.cdns.contains(&"unpkg".to_string()) {
                    stack.cdns.push("unpkg".to_string());
                }
            if (key_lower.contains("jsdelivr") || value_lower.contains("jsdelivr"))
                && !stack.cdns.contains(&"jsDelivr".to_string()) {
                    stack.cdns.push("jsDelivr".to_string());
                }
        }
    }

    fn detect_databases(&self, server_lower: Option<&str>, stack: &mut TechStack) {
        if let Some(s) = server_lower {
            if s.contains("mysql") && !stack.databases.contains(&"MySQL".to_string()) {
                stack.databases.push("MySQL".to_string());
            }
            if (s.contains("postgresql") || s.contains("postgres"))
                && !stack.databases.contains(&"PostgreSQL".to_string()) {
                    stack.databases.push("PostgreSQL".to_string());
                }
            if (s.contains("mongodb") || s.contains("mongo"))
                && !stack.databases.contains(&"MongoDB".to_string()) {
                    stack.databases.push("MongoDB".to_string());
                }
            if s.contains("redis") && !stack.databases.contains(&"Redis".to_string()) {
                stack.databases.push("Redis".to_string());
            }
            if s.contains("elasticsearch")
                && !stack.databases.contains(&"Elasticsearch".to_string())
            {
                stack.databases.push("Elasticsearch".to_string());
            }
            if s.contains("memcache") && !stack.databases.contains(&"Memcached".to_string()) {
                stack.databases.push("Memcached".to_string());
            }
        }
    }

    fn detect_javascript(&self, body: &str, stack: &mut TechStack) {
        if body.contains("node_modules/react") && !stack.javascript.contains(&"React".to_string()) {
            stack.javascript.push("React".to_string());
        }
        if body.contains("node_modules/vue") && !stack.javascript.contains(&"Vue.js".to_string()) {
            stack.javascript.push("Vue.js".to_string());
        }
        if body.contains("node_modules/angular")
            && !stack.javascript.contains(&"Angular".to_string())
        {
            stack.javascript.push("Angular".to_string());
        }
        if body.contains("jquery") && !stack.javascript.contains(&"jQuery".to_string()) {
            stack.javascript.push("jQuery".to_string());
        }
        if body.contains("prototype") && !stack.javascript.contains(&"Prototype".to_string()) {
            stack.javascript.push("Prototype".to_string());
        }
        if body.contains("dojo") && !stack.javascript.contains(&"Dojo".to_string()) {
            stack.javascript.push("Dojo".to_string());
        }
        if body.contains("backbone") && !stack.javascript.contains(&"Backbone.js".to_string()) {
            stack.javascript.push("Backbone.js".to_string());
        }
        if body.contains("underscore") && !stack.javascript.contains(&"Underscore.js".to_string()) {
            stack.javascript.push("Underscore.js".to_string());
        }
        if body.contains("lodash") && !stack.javascript.contains(&"Lodash".to_string()) {
            stack.javascript.push("Lodash".to_string());
        }
    }

    fn detect_languages(&self, powered_by_lower: Option<&str>, stack: &mut TechStack) {
        if let Some(pb) = powered_by_lower {
            if pb.contains("php") && !stack.languages.contains(&"PHP".to_string()) {
                stack.languages.push("PHP".to_string());
            }
            if pb.contains("ruby") && !stack.languages.contains(&"Ruby".to_string()) {
                stack.languages.push("Ruby".to_string());
            }
            if (pb.contains("python") || pb.contains("django") || pb.contains("flask"))
                && !stack.languages.contains(&"Python".to_string()) {
                    stack.languages.push("Python".to_string());
                }
            if (pb.contains("node") || pb.contains("express"))
                && !stack.languages.contains(&"Node.js".to_string()) {
                    stack.languages.push("Node.js".to_string());
                }
            if (pb.contains("java") || pb.contains("spring"))
                && !stack.languages.contains(&"Java".to_string()) {
                    stack.languages.push("Java".to_string());
                }
            if (pb.contains(".net") || pb.contains("asp"))
                && !stack.languages.contains(&"C#".to_string()) {
                    stack.languages.push("C#".to_string());
                }
            if (pb.contains("go") || pb.contains("golang"))
                && !stack.languages.contains(&"Go".to_string()) {
                    stack.languages.push("Go".to_string());
                }
            if pb.contains("rust")
                && !stack.languages.contains(&"Rust".to_string()) {
                    stack.languages.push("Rust".to_string());
                }
        }
    }
}

pub async fn detect_tech_stack(url: &str) -> Result<TechDetectionResult> {
    let detector = TechDetector::new()?;
    detector.detect(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_stack_default() {
        let stack = TechStack::default();
        assert!(stack.servers.is_empty());
        assert!(stack.frameworks.is_empty());
        assert!(stack.languages.is_empty());
        assert!(stack.databases.is_empty());
        assert!(stack.cdns.is_empty());
        assert!(stack.cms.is_empty());
        assert!(stack.javascript.is_empty());
        assert!(stack.other.is_empty());
    }

    #[test]
    fn test_tech_stack_serialization() {
        let mut stack = TechStack::default();
        stack.servers.push("Nginx".to_string());
        stack.frameworks.push("Express".to_string());
        stack.languages.push("Node.js".to_string());
        let json = serde_json::to_string(&stack).unwrap();
        assert!(json.contains("Nginx"));
        assert!(json.contains("Express"));
        let decoded: TechStack = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.servers.len(), 1);
        assert_eq!(decoded.servers[0], "Nginx");
    }

    #[test]
    fn test_tech_detection_result_serialization() {
        let result = TechDetectionResult {
            url: "https://example.com".to_string(),
            status_code: 200,
            headers: FxHashMap::from_iter([
                ("server".to_string(), "nginx".to_string()),
                ("x-powered-by".to_string(), "Express".to_string()),
            ]),
            tech_stack: TechStack::default(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("200"));
        let decoded: TechDetectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.url, "https://example.com");
        assert_eq!(decoded.status_code, 200);
    }

    #[test]
    fn test_tech_detector_new() {
        let detector = TechDetector::new();
        assert!(detector.is_ok());
    }

    #[test]
    fn test_tech_stack_clone() {
        let mut stack = TechStack::default();
        stack.servers.push("Apache".to_string());
        let cloned = stack.clone();
        assert_eq!(cloned.servers.len(), 1);
        assert_eq!(cloned.servers[0], "Apache");
    }

    #[test]
    fn test_tech_detection_result_clone() {
        let result = TechDetectionResult {
            url: "https://example.com".to_string(),
            status_code: 404,
            headers: FxHashMap::default(),
            tech_stack: TechStack::default(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.url, "https://example.com");
        assert_eq!(cloned.status_code, 404);
    }

    #[test]
    fn test_tech_stack_multiple_categories() {
        let mut stack = TechStack::default();
        stack.servers.push("nginx".to_string());
        stack.frameworks.push("django".to_string());
        stack.languages.push("python".to_string());
        stack.databases.push("postgresql".to_string());
        stack.cdns.push("cloudflare".to_string());
        stack.cms.push("wordpress".to_string());
        stack.javascript.push("react".to_string());

        assert_eq!(stack.servers.len(), 1);
        assert_eq!(stack.frameworks.len(), 1);
        assert_eq!(stack.languages.len(), 1);
        assert_eq!(stack.databases.len(), 1);
        assert_eq!(stack.cdns.len(), 1);
        assert_eq!(stack.cms.len(), 1);
        assert_eq!(stack.javascript.len(), 1);
    }
}
