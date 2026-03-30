use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::utils::create_insecure_http_client;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentDiscovery {
    pub url: String,
    pub discovered: Vec<DiscoveredContent>,
    pub sensitive_files: Vec<SensitiveFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredContent {
    pub url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub is_sensitive: bool,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveFile {
    pub url: String,
    pub file_type: String,
    pub severity: String,
    pub description: String,
}

pub struct ContentScanner {
    client: reqwest::Client,
    concurrency: usize,
}

impl ContentScanner {
    pub fn new(concurrency: usize) -> Result<Self> {
        let client = create_insecure_http_client(10)?;

        Ok(Self {
            client,
            concurrency,
        })
    }

    pub async fn scan(&self, base_url: &str) -> Result<ContentDiscovery> {
        let sensitive_paths = self.get_sensitive_paths();

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for path in sensitive_paths {
            let url = format!("{}{}", base_url.trim_end_matches('/'), path);
            let client = self.client.clone();
            let semaphore = Arc::clone(&semaphore);

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.ok();

                match client.get(&url).send().await {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let content_type = response
                            .headers()
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .map(|s| s.to_string());

                        if status == 200 || status == 401 || status == 403 {
                            let (category, _severity) = Self::categorize_path(path);
                            let is_sensitive = !category.is_empty();

                            Some(DiscoveredContent {
                                url,
                                status_code: status,
                                content_type,
                                is_sensitive,
                                category,
                            })
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            });

            handles.push(handle);
        }

        let mut discovered = Vec::new();
        let mut sensitive = Vec::new();

        for handle in handles {
            if let Ok(Some(content)) = handle.await {
                if content.is_sensitive {
                    sensitive.push(SensitiveFile {
                        url: content.url.clone(),
                        file_type: Self::get_file_type(&content.url),
                        severity: content.category.clone(),
                        description: Self::get_description(&content.url),
                    });
                }
                discovered.push(content);
            }
        }

        Ok(ContentDiscovery {
            url: base_url.to_string(),
            discovered,
            sensitive_files: sensitive,
        })
    }

    fn get_sensitive_paths(&self) -> Vec<&'static str> {
        vec![
            "/.env",
            "/.git/config",
            "/.git/HEAD",
            "/.gitignore",
            "/.svn/entries",
            "/.hg/requires",
            "/composer.json",
            "/package.json",
            "/package-lock.json",
            "/yarn.lock",
            "/Gemfile",
            "/Gemfile.lock",
            "/requirements.txt",
            "/Pipfile",
            "/setup.py",
            "/Cargo.toml",
            "/Cargo.lock",
            "/pom.xml",
            "/build.gradle",
            "/.aws/credentials",
            "/.aws/config",
            "/id_rsa",
            "/id_rsa.pub",
            "/.pem",
            "/.key",
            "/.htaccess",
            "/.htpasswd",
            "/wp-config.php",
            "/configuration.php",
            "/config.php",
            "/settings.py",
            "/database.yml",
            "/credentials.json",
            "/secrets.yml",
            "/.env.local",
            "/.env.production",
            "/.env.development",
            "/debug",
            "/phpinfo.php",
            "/info.php",
            "/server-status",
            "/server-info",
            "/actuator/health",
            "/actuator/env",
            "/actuator/configprops",
            "/swagger-ui.html",
            "/swagger-ui/",
            "/api/docs",
            "/v2/api-docs",
            "/graphql",
            "/graphiql",
            "/console",
            "/admin",
            "/administrator",
            "/login",
            "/backup",
            "/backups",
            "/db",
            "/database",
            "/sql",
            "/dump.sql",
            "/.sql",
            "/.log",
            "/logs",
            "/temp",
            "/tmp",
            "/cache",
            "/api",
            "/api/v1",
            "/api/v2",
            "/rest",
            "/soap",
            "/xmlrpc.php",
            "/server.php",
            "/index.php",
            "/.DS_Store",
            "/.metadata_never_index",
            "/thumbs.db",
            "/desktop.ini",
        ]
    }

    fn categorize_path(path: &str) -> (String, String) {
        let path_lower = path.to_lowercase();

        if path_lower.contains(".env")
            || path_lower.contains("credentials")
            || path_lower.contains("secrets")
            || path_lower.contains("id_rsa")
            || path_lower.contains(".pem")
            || path_lower.contains(".key")
            || path_lower.contains(".aws")
        {
            return ("credentials".to_string(), "critical".to_string());
        }

        if path_lower.contains(".git") || path_lower.contains(".svn") || path_lower.contains(".hg")
        {
            return ("source_control".to_string(), "high".to_string());
        }

        if path_lower.contains("config")
            || path_lower.contains("composer")
            || path_lower.contains("package")
            || path_lower.contains("requirements")
            || path_lower.contains("cargo")
        {
            return ("config".to_string(), "medium".to_string());
        }

        if path_lower.contains("dump")
            || path_lower.contains(".sql")
            || path_lower.contains("database")
        {
            return ("database".to_string(), "critical".to_string());
        }

        if path_lower.contains("log")
            || path_lower.contains("temp")
            || path_lower.contains("tmp")
            || path_lower.contains("cache")
        {
            return ("sensitive".to_string(), "medium".to_string());
        }

        if path_lower.contains("admin")
            || path_lower.contains("console")
            || path_lower.contains("swagger")
            || path_lower.contains("actuator")
        {
            return ("admin_interface".to_string(), "high".to_string());
        }

        (String::new(), String::new())
    }

    fn get_file_type(url: &str) -> String {
        if url.contains(".env") {
            "Environment File".to_string()
        } else if url.contains(".git") {
            "Git Repository".to_string()
        } else if url.contains(".json") {
            "JSON File".to_string()
        } else if url.contains(".xml") {
            "XML File".to_string()
        } else if url.contains(".sql") {
            "SQL Dump".to_string()
        } else if url.contains(".log") {
            "Log File".to_string()
        } else if url.contains(".php") {
            "PHP File".to_string()
        } else if url.contains(".yml") || url.contains(".yaml") {
            "YAML File".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn get_description(url: &str) -> String {
        let url_lower = url.to_lowercase();

        if url_lower.contains(".env") {
            "Environment variables file may contain sensitive credentials".to_string()
        } else if url_lower.contains(".git") {
            "Git repository files may expose source code and commit history".to_string()
        } else if url_lower.contains("id_rsa") || url_lower.contains(".pem") {
            "Private SSH key or certificate".to_string()
        } else if url_lower.contains(".aws") {
            "AWS credentials configuration".to_string()
        } else if url_lower.contains(".sql") || url_lower.contains("dump") {
            "Database backup may contain sensitive data".to_string()
        } else if url_lower.contains("config") {
            "Configuration file may expose system internals".to_string()
        } else if url_lower.contains("composer") || url_lower.contains("package") {
            "Dependency configuration may reveal used libraries and versions".to_string()
        } else if url_lower.contains("log") {
            "Log file may contain sensitive information".to_string()
        } else if url_lower.contains("admin") || url_lower.contains("console") {
            "Administrative interface may be accessible".to_string()
        } else {
            "Potentially sensitive file discovered".to_string()
        }
    }
}

pub async fn scan_content(base_url: &str, concurrency: usize) -> Result<ContentDiscovery> {
    let scanner = ContentScanner::new(concurrency)?;
    scanner.scan(base_url).await
}
