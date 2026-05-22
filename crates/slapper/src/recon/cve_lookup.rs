use rustc_hash::FxHashMap;
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
    cve_cache: FxHashMap<String, Vec<CveEntry>>,
}

impl CveEngine {
    pub fn new() -> Self {
        Self {
            cve_cache: FxHashMap::default(),
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
            cves.reserve(vulns.len().min(20));
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

    #[allow(dead_code)]
    /// ExploitDB lookup - implementation incomplete, returns empty
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvss_severity_parse() {
        assert_eq!(
            CveEngine::parse_severity("CRITICAL"),
            CvssSeverity::Critical
        );
        assert_eq!(CveEngine::parse_severity("HIGH"), CvssSeverity::High);
        assert_eq!(CveEngine::parse_severity("MEDIUM"), CvssSeverity::Medium);
        assert_eq!(CveEngine::parse_severity("LOW"), CvssSeverity::Low);
        assert_eq!(CveEngine::parse_severity("UNKNOWN"), CvssSeverity::None);
        assert_eq!(CveEngine::parse_severity(""), CvssSeverity::None);
    }

    #[test]
    fn test_cvss_severity_case_insensitive() {
        assert_eq!(
            CveEngine::parse_severity("critical"),
            CvssSeverity::Critical
        );
        assert_eq!(CveEngine::parse_severity("High"), CvssSeverity::High);
        assert_eq!(CveEngine::parse_severity("MeDiUm"), CvssSeverity::Medium);
    }

    #[test]
    fn test_cve_entry_serialization() {
        let entry = CveEntry {
            id: "CVE-2021-41773".to_string(),
            description: "Path traversal vulnerability".to_string(),
            severity: CvssSeverity::Critical,
            cvss_score: 10.0,
            published: "2021-10-04T00:00:00Z".to_string(),
            affected_products: vec!["Apache".to_string()],
            references: vec!["https://nvd.nist.gov".to_string()],
            cwe: Some("CWE-22".to_string()),
            has_exploit: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("CVE-2021-41773"));
        assert!(json.contains("Critical"));
        let decoded: CveEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "CVE-2021-41773");
        assert_eq!(decoded.cvss_score, 10.0);
        assert!(decoded.has_exploit);
    }

    #[test]
    fn test_technology_match_serialization() {
        let tech = TechnologyMatch {
            name: "nginx".to_string(),
            version: Some("1.20.0".to_string()),
            cves: vec![CveEntry {
                id: "CVE-2021-23017".to_string(),
                description: "Off-by-one error".to_string(),
                severity: CvssSeverity::High,
                cvss_score: 7.5,
                published: "2021-06-01".to_string(),
                affected_products: vec!["nginx".to_string()],
                references: vec![],
                cwe: None,
                has_exploit: false,
            }],
        };
        let json = serde_json::to_string(&tech).unwrap();
        let decoded: TechnologyMatch = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, "nginx");
        assert_eq!(decoded.version.as_deref(), Some("1.20.0"));
        assert_eq!(decoded.cves.len(), 1);
    }

    #[test]
    fn test_cve_engine_new() {
        let engine = CveEngine::new();
        assert!(engine.cve_cache.is_empty());
    }

    #[test]
    fn test_cve_entry_clone() {
        let entry = CveEntry {
            id: "CVE-2023-0001".to_string(),
            description: "Test".to_string(),
            severity: CvssSeverity::High,
            cvss_score: 7.5,
            published: String::new(),
            affected_products: vec![],
            references: vec![],
            cwe: None,
            has_exploit: false,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.id, "CVE-2023-0001");
        assert_eq!(cloned.severity, CvssSeverity::High);
    }

    #[test]
    fn test_exploit_db_entry_serialization() {
        let entry = ExploitDbEntry {
            id: "EDB-12345".to_string(),
            description: "Test exploit".to_string(),
            exploit_type: "web".to_string(),
            platform: "linux".to_string(),
            author: "tester".to_string(),
            date: "2023-01-01".to_string(),
            url: "https://exploit-db.com/12345".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let decoded: ExploitDbEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "EDB-12345");
        assert_eq!(decoded.platform, "linux");
    }

    #[test]
    fn test_cve_engine_match_technology() {
        let mut engine = CveEngine::new();
        let matches = engine.match_technology_cves("apache", Some("2.4.49"));
        assert!(matches.is_empty());
    }
}
