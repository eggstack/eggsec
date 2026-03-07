#![allow(dead_code)]

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDiff {
    pub baseline: ResponseSnapshot,
    pub diff: DiffResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSnapshot {
    pub status_code: u16,
    pub headers: HeaderSnapshot,
    pub body_hash: String,
    pub body_length: usize,
    pub content_type: Option<String>,
    pub timing_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderSnapshot {
    pub headers: Vec<(String, String)>,
    pub etag: Option<String>,
    pub set_cookie: Vec<String>,
    pub cache_control: Option<String>,
    pub content_length: Option<String>,
    pub x_powered_by: Option<String>,
    pub server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub status_changed: bool,
    pub content_type_changed: bool,
    pub body_length_diff: isize,
    pub new_headers: Vec<String>,
    pub removed_headers: Vec<String>,
    pub header_value_changes: Vec<HeaderChange>,
    pub new_cookies: Vec<String>,
    pub timing_change_ms: i64,
    pub anomaly_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderChange {
    pub name: String,
    pub baseline: Option<String>,
    pub current: Option<String>,
}

impl Default for DiffResult {
    fn default() -> Self {
        Self {
            status_changed: false,
            content_type_changed: false,
            body_length_diff: 0,
            new_headers: Vec::new(),
            removed_headers: Vec::new(),
            header_value_changes: Vec::new(),
            new_cookies: Vec::new(),
            timing_change_ms: 0,
            anomaly_score: 0.0,
        }
    }
}

pub struct ResponseDiffer {
    baseline: Option<ResponseSnapshot>,
    ignore_headers: HashSet<String>,
    ignore_body_patterns: Vec<String>,
    min_anomaly_threshold: f64,
}

impl Default for ResponseDiffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseDiffer {
    pub fn new() -> Self {
        let mut ignore_headers = HashSet::new();
        ignore_headers.insert("date".to_string());
        ignore_headers.insert("content-length".to_string());
        ignore_headers.insert("connection".to_string());
        ignore_headers.insert("keep-alive".to_string());

        Self {
            baseline: None,
            ignore_headers,
            ignore_body_patterns: Vec::new(),
            min_anomaly_threshold: 0.3,
        }
    }

    pub fn with_ignore_headers(mut self, headers: Vec<String>) -> Self {
        for h in headers {
            self.ignore_headers.insert(h);
        }
        self
    }

    pub fn with_body_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_body_patterns = patterns;
        self
    }

    pub fn set_baseline(&mut self, snapshot: ResponseSnapshot) {
        self.baseline = Some(snapshot);
    }

    pub fn capture_baseline(
        &mut self,
        status_code: u16,
        headers: &HeaderMap,
        body: &[u8],
        timing_ms: u64,
    ) -> ResponseSnapshot {
        let snapshot = self.create_snapshot(status_code, headers, body, timing_ms);
        self.baseline = Some(snapshot.clone());
        snapshot
    }

    pub fn diff(
        &self,
        status_code: u16,
        headers: &HeaderMap,
        body: &[u8],
        timing_ms: u64,
    ) -> ResponseDiff {
        let current = self.create_snapshot(status_code, headers, body, timing_ms);

        let diff = match &self.baseline {
            Some(baseline) => self.compute_diff(baseline, &current),
            None => DiffResult::default(),
        };

        ResponseDiff {
            baseline: current,
            diff,
        }
    }

    fn create_snapshot(
        &self,
        status_code: u16,
        headers: &HeaderMap,
        body: &[u8],
        timing_ms: u64,
    ) -> ResponseSnapshot {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(body);
        let body_hash = format!("{:x}", hasher.finalize());

        let mut header_snaps = Vec::new();
        let mut etag = None;
        let mut set_cookie = Vec::new();
        let mut cache_control = None;
        let mut content_length = None;
        let mut x_powered_by = None;
        let mut server = None;

        for (name, value) in headers.iter() {
            let name_str = name.as_str().to_lowercase();
            let value_str = value.to_str().unwrap_or("").to_string();

            if !self.ignore_headers.contains(&name_str) {
                header_snaps.push((name.as_str().to_string(), value_str.clone()));
            }

            match name_str.as_str() {
                "etag" => etag = Some(value_str),
                "set-cookie" => set_cookie.push(value_str),
                "cache-control" => cache_control = Some(value_str),
                "content-length" => content_length = Some(value_str),
                "x-powered-by" => x_powered_by = Some(value_str),
                "server" => server = Some(value_str),
                _ => {}
            }
        }

        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        ResponseSnapshot {
            status_code,
            headers: HeaderSnapshot {
                headers: header_snaps,
                etag,
                set_cookie,
                cache_control,
                content_length,
                x_powered_by,
                server,
            },
            body_hash,
            body_length: body.len(),
            content_type,
            timing_ms,
        }
    }

    fn compute_diff(&self, baseline: &ResponseSnapshot, current: &ResponseSnapshot) -> DiffResult {
        let mut diff = DiffResult::default();

        if baseline.status_code != current.status_code {
            diff.status_changed = true;
            diff.anomaly_score += 0.3;
        }

        if baseline.content_type != current.content_type {
            diff.content_type_changed = true;
            diff.anomaly_score += 0.2;
        }

        let body_diff = current.body_length as isize - baseline.body_length as isize;
        diff.body_length_diff = body_diff;

        if !(-1000..=1000).contains(&body_diff) {
            diff.anomaly_score += 0.2;
        }

        let baseline_headers: HashSet<_> = baseline
            .headers
            .headers
            .iter()
            .map(|(k, _)| k.clone())
            .collect();
        let current_headers: HashSet<_> = current
            .headers
            .headers
            .iter()
            .map(|(k, _)| k.clone())
            .collect();

        for h in current_headers.difference(&baseline_headers) {
            diff.new_headers.push(h.clone());
            diff.anomaly_score += 0.1;
        }

        for h in baseline_headers.difference(&current_headers) {
            diff.removed_headers.push(h.clone());
            diff.anomaly_score += 0.1;
        }

        let baseline_header_map: HashMap<_, _> = baseline
            .headers
            .headers
            .iter()
            .map(|(k, v)| (k, v))
            .collect();
        let current_header_map: HashMap<_, _> = current
            .headers
            .headers
            .iter()
            .map(|(k, v)| (k, v))
            .collect();

        for (name, baseline_val) in &baseline_header_map {
            if let Some(current_val) = current_header_map.get(name) {
                if baseline_val != current_val {
                    diff.header_value_changes.push(HeaderChange {
                        name: (*name).clone(),
                        baseline: Some((*baseline_val).clone()),
                        current: Some((*current_val).clone()),
                    });
                    diff.anomaly_score += 0.05;
                }
            }
        }

        let baseline_cookies: HashSet<_> = baseline.headers.set_cookie.iter().collect();
        let current_cookies: HashSet<_> = current.headers.set_cookie.iter().collect();

        for c in current_cookies.difference(&baseline_cookies) {
            diff.new_cookies.push((*c).clone());
            diff.anomaly_score += 0.15;
        }

        let timing_diff = current.timing_ms as i64 - baseline.timing_ms as i64;
        diff.timing_change_ms = timing_diff;

        if timing_diff > 1000 {
            diff.anomaly_score += 0.2;
        }

        diff
    }

    pub fn is_anomaly(&self, diff: &DiffResult) -> bool {
        diff.anomaly_score >= self.min_anomaly_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_differ() {
        let mut differ = ResponseDiffer::new();

        let baseline = ResponseSnapshot {
            status_code: 200,
            headers: HeaderSnapshot {
                headers: vec![],
                etag: None,
                set_cookie: vec![],
                cache_control: None,
                content_length: None,
                x_powered_by: None,
                server: None,
            },
            body_hash: "abc".to_string(),
            body_length: 100,
            content_type: Some("text/html".to_string()),
            timing_ms: 50,
        };

        differ.set_baseline(baseline);

        assert!(differ.baseline.is_some());
    }
}
