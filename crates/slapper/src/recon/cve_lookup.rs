use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CveEntry {
    pub id: String,
    pub description: String,
    pub severity: CvssSeverity,
    pub cvss_score: f32,
    pub published: String,
    pub affected_products: Vec<String>,
    pub references: Vec<String>,
    pub cwe: Option<String>,
    pub has_exploit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CvssSeverity {
    Critical,
    High,
    Medium,
    Low,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyMatch {
    pub name: String,
    pub version: Option<String>,
    pub cves: Vec<CveEntry>,
}

pub struct CveEngine {
    cve_cache: std::collections::HashMap<String, Vec<CveEntry>>,
}

impl CveEngine {
    pub fn new() -> Self {
        Self {
            cve_cache: std::collections::HashMap::new(),
        }
    }

    pub fn lookup_cve(
        &mut self,
        cve_id: &str,
    ) -> Result<CveEntry, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(cached) = self.cve_cache.get(cve_id) {
            return Ok(cached.clone());
        }

        let url = format!(
            "https://services.nvd.nist.gov/rest/json/cves/2.0?cveId={}",
            cve_id
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()?;

        if !response.status().is_success() {
            return Err(format!("CVE lookup failed: {}", response.status()).into());
        }

        let json: serde_json::Value = response.json()?;

        let cve = self.parse_nvd_response(cve_id, &json)?;

        self.cve_cache.insert(cve_id.to_string(), cve.clone());

        Ok(cve)
    }

    fn parse_nvd_response(
        &self,
        cve_id: &str,
        json: &serde_json::Value,
    ) -> Result<CveEntry, Box<dyn std::error::Error + Send + Sync>> {
        let vuln = json
            .get("vulnerabilities")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("cve"));

        let description = vuln
            .and_then(|v| v.get("descriptions"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let metrics = vuln.and_then(|v| v.get("metrics"));

        let (cvss_score, severity) = if let Some(metrics) = metrics {
            if let Some(cvss31) = metrics
                .get("cvssMetricV31")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
            {
                let score = cvss31
                    .get("cvssData")
                    .and_then(|d| d.get("baseScore"))
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0) as f32;
                let severity_str = cvss31
                    .get("cvssData")
                    .and_then(|d| d.get("baseSeverity"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("UNKNOWN");

                (score, Self::parse_severity(severity_str))
            } else {
                (0.0, CvssSeverity::None)
            }
        } else {
            (0.0, CvssSeverity::None)
        };

        let published = vuln
            .and_then(|v| v.get("published"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let references = vuln
            .and_then(|v| v.get("references"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| r.get("url").and_then(|u| u.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let cwe = vuln
            .and_then(|v| v.get("weaknesses"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|w| w.get("description"))
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|d| d.get("value"))
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(CveEntry {
            id: cve_id.to_string(),
            description,
            severity,
            cvss_score,
            published,
            affected_products: vec![],
            references,
            cwe,
            has_exploit: false,
        })
    }

    fn parse_severity(s: &str) -> CvssSeverity {
        match s.to_uppercase().as_str() {
            "CRITICAL" => CvssSeverity::Critical,
            "HIGH" => CvssSeverity::High,
            "MEDIUM" => CvssSeverity::Medium,
            "LOW" => CvssSeverity::Low,
            _ => CvssSeverity::None,
        }
    }

    pub fn match_technology_cves(
        &mut self,
        technology: &str,
        version: Option<&str>,
    ) -> Vec<TechnologyMatch> {
        let mut matches = Vec::new();

        let query = if let Some(ver) = version {
            format!("{} {}", technology, ver)
        } else {
            technology.to_string()
        };

        let url = format!(
            "https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch={}",
            urlencoding::encode(&query)
        );

        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
        {
            if let Ok(response) = client.get(&url).header("Accept", "application/json").send() {
                if let Ok(json) = response.json::<serde_json::Value>() {
                    let cves = self.parse_cve_list(&json);

                    if !cves.is_empty() {
                        matches.push(TechnologyMatch {
                            name: technology.to_string(),
                            version: version.map(String::from),
                            cves,
                        });
                    }
                }
            }
        }

        matches
    }

    fn parse_cve_list(&self, json: &serde_json::Value) -> Vec<CveEntry> {
        let mut cves = Vec::new();

        if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.as_array()) {
            for vuln in vulns.iter().take(20) {
                if let Some(cve) = vuln.get("cve") {
                    let id = cve
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let description = cve
                        .get("descriptions")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.get("value"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let (cvss_score, severity) = if let Some(metrics) = cve.get("metrics") {
                        if let Some(cvss31) = metrics
                            .get("cvssMetricV31")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first())
                        {
                            let score = cvss31
                                .get("cvssData")
                                .and_then(|d| d.get("baseScore"))
                                .and_then(|s| s.as_f64())
                                .unwrap_or(0.0) as f32;
                            let severity_str = cvss31
                                .get("cvssData")
                                .and_then(|d| d.get("baseSeverity"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("UNKNOWN");
                            (score, Self::parse_severity(severity_str))
                        } else {
                            (0.0, CvssSeverity::None)
                        }
                    } else {
                        (0.0, CvssSeverity::None)
                    };

                    let published = cve
                        .get("published")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    cves.push(CveEntry {
                        id,
                        description,
                        severity,
                        cvss_score,
                        published,
                        affected_products: vec![],
                        references: vec![],
                        cwe: None,
                        has_exploit: false,
                    });
                }
            }
        }

        cves
    }

    pub fn get_exploit_db_exploits(&self, cve_id: &str) -> Vec<ExploitDbEntry> {
        vec![]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploitDbEntry {
    pub id: String,
    pub description: String,
    pub exploit_type: String,
    pub platform: String,
    pub author: String,
    pub date: String,
    pub url: String,
}

pub fn lookup_cve(cve_id: &str) -> Result<CveEntry, Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = CveEngine::new();
    engine.lookup_cve(cve_id)
}
