use super::lifecycle::{FindingStatus, ScanRun, StoredFinding};
use super::Finding;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Local JSONL-based finding store
pub struct FindingStore {
    base_dir: PathBuf,
}

impl FindingStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn findings_path(&self) -> PathBuf {
        self.base_dir.join("findings.jsonl")
    }

    fn runs_path(&self) -> PathBuf {
        self.base_dir.join("scan_runs.jsonl")
    }

    /// Ensure the store directory exists
    pub fn init(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    /// Store a finding
    pub fn store_finding(&self, finding: Finding) -> anyhow::Result<StoredFinding> {
        let fingerprint = finding.fingerprint.clone();
        let mut findings = self.load_findings()?;

        if let Some(idx) = findings.iter().position(|f| f.finding.fingerprint == fingerprint) {
            findings[idx].finding = finding;
            self.write_findings(&findings)?;
            return Ok(findings[idx].clone());
        }

        let stored = StoredFinding::new(finding);
        let line = serde_json::to_string(&stored)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.findings_path())?;

        writeln!(file, "{}", line)?;
        Ok(stored)
    }

    /// Load all findings
    pub fn load_findings(&self) -> anyhow::Result<Vec<StoredFinding>> {
        let path = self.findings_path();
        if !path.exists() {
            return Ok(vec![]);
        }

        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut findings = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let stored: StoredFinding = serde_json::from_str(&line)?;
            findings.push(stored);
        }

        Ok(findings)
    }

    /// Update finding status
    pub fn update_status(
        &self,
        fingerprint: &str,
        new_status: FindingStatus,
        note: Option<String>,
    ) -> anyhow::Result<()> {
        let mut findings = self.load_findings()?;

        let finding = findings
            .iter_mut()
            .find(|f| f.finding.fingerprint == fingerprint);

        match finding {
            Some(f) => {
                f.change_status(new_status, note);
            }
            None => {
                anyhow::bail!("Finding with fingerprint '{}' not found", fingerprint);
            }
        }

        self.write_findings(&findings)?;
        Ok(())
    }

    /// Get findings by status
    pub fn findings_by_status(&self, status: FindingStatus) -> anyhow::Result<Vec<StoredFinding>> {
        let all = self.load_findings()?;
        Ok(all.into_iter().filter(|f| f.status == status).collect())
    }

    /// Record a scan run
    pub fn record_run(&self, run: ScanRun) -> anyhow::Result<()> {
        let line = serde_json::to_string(&run)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.runs_path())?;

        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Load all scan runs
    pub fn load_runs(&self) -> anyhow::Result<Vec<ScanRun>> {
        let path = self.runs_path();
        if !path.exists() {
            return Ok(vec![]);
        }

        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut runs = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let run: ScanRun = serde_json::from_str(&line)?;
            runs.push(run);
        }

        Ok(runs)
    }

    fn write_findings(&self, findings: &[StoredFinding]) -> anyhow::Result<()> {
        let mut file = fs::File::create(self.findings_path())?;
        for finding in findings {
            let line = serde_json::to_string(finding)?;
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::*;
    use chrono::Utc;

    fn test_store() -> FindingStore {
        let dir = std::env::temp_dir().join(format!("slapper_test_{}", rand::random::<u64>()));
        FindingStore::new(dir)
    }

    fn test_finding(fingerprint: &str) -> Finding {
        Finding {
            id: format!("test-{}", fingerprint),
            fingerprint: fingerprint.to_string(),
            title: "Test Finding".to_string(),
            description: "Test".to_string(),
            severity: crate::types::Severity::Medium,
            confidence: Confidence::High,
            finding_type: FindingType::Vulnerability,
            cwe: None,
            owasp: None,
            cve: None,
            affected_asset: AffectedAsset {
                asset_type: "web_application".to_string(),
                identifier: "https://example.com".to_string(),
                host: Some("example.com".to_string()),
                port: Some(443),
                protocol: Some("https".to_string()),
            },
            location: FindingLocation {
                url: None,
                path: None,
                parameter: None,
                header: None,
                method: None,
                line: None,
                file: None,
            },
            evidence: vec![],
            reproduction: None,
            remediation: None,
            discovered_at: Utc::now(),
            source: FindingSource {
                tool: "test".to_string(),
                module: "test".to_string(),
                run_id: None,
            },
            tags: vec![],
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn store_and_load_finding() {
        let store = test_store();
        store.init().unwrap();

        let stored = store.store_finding(test_finding("fp1")).unwrap();
        assert_eq!(stored.status, FindingStatus::New);

        let findings = store.load_findings().unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding.fingerprint, "fp1");
    }

    #[test]
    fn update_finding_status() {
        let store = test_store();
        store.init().unwrap();

        store.store_finding(test_finding("fp1")).unwrap();
        store
            .update_status(
                "fp1",
                FindingStatus::Confirmed,
                Some("Looks real".to_string()),
            )
            .unwrap();

        let findings = store.load_findings().unwrap();
        assert_eq!(findings[0].status, FindingStatus::Confirmed);
        assert_eq!(findings[0].status_history.len(), 1);
    }

    #[test]
    fn findings_by_status() {
        let store = test_store();
        store.init().unwrap();

        store.store_finding(test_finding("fp1")).unwrap();
        store.store_finding(test_finding("fp2")).unwrap();
        store
            .update_status("fp1", FindingStatus::Confirmed, None)
            .unwrap();

        let confirmed = store.findings_by_status(FindingStatus::Confirmed).unwrap();
        assert_eq!(confirmed.len(), 1);

        let new = store.findings_by_status(FindingStatus::New).unwrap();
        assert_eq!(new.len(), 1);
    }

    #[test]
    fn record_and_load_run() {
        let store = test_store();
        store.init().unwrap();

        let run = ScanRun {
            id: "run-1".to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            target: "https://example.com".to_string(),
            findings_count: 5,
            new_findings_count: 3,
            resolved_findings_count: 1,
        };

        store.record_run(run).unwrap();
        let runs = store.load_runs().unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, "run-1");
    }
}
