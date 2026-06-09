//! Normalized request corpus types for browser crawl results.
//!
//! Captures requests observed during browser-based crawling and normalizes
//! them into a corpus that can feed scanner and fuzzer workflows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A normalized request corpus entry captured from browser crawling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusEntry {
    pub url: String,
    pub method: String,
    pub headers: Vec<CorpusHeader>,
    pub body_shape: Option<BodyShape>,
    pub content_type: Option<String>,
    pub source: RequestSource,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusHeader {
    pub name: String,
    pub value: String,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyShape {
    pub content_type: Option<String>,
    pub fields: Vec<BodyField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestSource {
    Xhr,
    Fetch,
    Form,
    Navigation,
    WebSocket,
    Script,
    Other,
}

/// Complete corpus from a browser crawl session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCorpus {
    pub entries: Vec<CorpusEntry>,
    pub urls: Vec<String>,
    pub api_endpoints: Vec<String>,
    pub forms: Vec<FormInfo>,
    pub websocket_urls: Vec<String>,
    pub javascript_urls: Vec<String>,
    pub graphql_candidates: Vec<String>,
    pub openapi_links: Vec<String>,
    pub crawl_duration_ms: u64,
    pub pages_visited: usize,
    #[serde(skip)]
    seen_keys: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub action: String,
    pub method: String,
    pub fields: Vec<String>,
}

impl RequestCorpus {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            urls: Vec::new(),
            api_endpoints: Vec::new(),
            forms: Vec::new(),
            websocket_urls: Vec::new(),
            javascript_urls: Vec::new(),
            graphql_candidates: Vec::new(),
            openapi_links: Vec::new(),
            crawl_duration_ms: 0,
            pages_visited: 0,
            seen_keys: HashSet::new(),
        }
    }

    /// Add a corpus entry, deduplicating by URL + method
    pub fn add_entry(&mut self, entry: CorpusEntry) {
        let key = format!("{}:{}", entry.method, entry.url);
        if self.seen_keys.insert(key) {
            self.entries.push(entry);
        }
    }

    /// Get all unique API-like endpoints (paths with parameters, query strings, etc.)
    pub fn api_endpoints(&self) -> Vec<&CorpusEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.url.contains('?')
                    || e.url.contains("/api/")
                    || e.url.contains("/graphql")
                    || e.method != "GET"
            })
            .collect()
    }

    /// Export as JSON
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

impl Default for RequestCorpus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(url: &str, method: &str) -> CorpusEntry {
        CorpusEntry {
            url: url.to_string(),
            method: method.to_string(),
            headers: vec![],
            body_shape: None,
            content_type: None,
            source: RequestSource::Fetch,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn corpus_deduplicates_entries() {
        let mut corpus = RequestCorpus::new();
        corpus.add_entry(make_entry("https://example.com/api/users", "GET"));
        corpus.add_entry(make_entry("https://example.com/api/users", "GET"));
        corpus.add_entry(make_entry("https://example.com/api/users", "POST"));

        assert_eq!(corpus.entries.len(), 2);
    }

    #[test]
    fn api_endpoints_filter() {
        let mut corpus = RequestCorpus::new();
        corpus.add_entry(make_entry("https://example.com/", "GET"));
        corpus.add_entry(make_entry("https://example.com/api/users", "GET"));
        corpus.add_entry(make_entry("https://example.com/page?q=test", "GET"));
        corpus.add_entry(make_entry("https://example.com/other", "POST"));

        let api = corpus.api_endpoints();
        assert_eq!(api.len(), 3);
    }

    #[test]
    fn corpus_serializes_to_json() {
        let mut corpus = RequestCorpus::new();
        corpus.add_entry(make_entry("https://example.com/", "GET"));
        let json = corpus.to_json().unwrap();
        assert!(json.contains("example.com"));
    }

    #[test]
    fn default_corpus_is_empty() {
        let corpus = RequestCorpus::default();
        assert!(corpus.entries.is_empty());
        assert_eq!(corpus.pages_visited, 0);
    }

    #[test]
    fn request_source_serializes_snake_case() {
        let source = RequestSource::WebSocket;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"web_socket\"");
    }
}
