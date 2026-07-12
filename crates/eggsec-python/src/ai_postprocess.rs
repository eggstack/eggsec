use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

use crate::runtime_sync;

#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiProviderPy {
    OpenAI,
    Azure,
    Anthropic,
    OpenAICompatible,
}

#[pymethods]
impl AiProviderPy {
    fn __repr__(&self) -> String {
        format!("AiProvider.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl AiProviderPy {
    fn as_str(&self) -> &str {
        match self {
            AiProviderPy::OpenAI => "OpenAI",
            AiProviderPy::Azure => "Azure",
            AiProviderPy::Anthropic => "Anthropic",
            AiProviderPy::OpenAICompatible => "OpenAICompatible",
        }
    }

    fn from_engine(engine: eggsec::ai::Provider) -> Self {
        match engine {
            eggsec::ai::Provider::OpenAI => AiProviderPy::OpenAI,
            eggsec::ai::Provider::Azure => AiProviderPy::Azure,
            eggsec::ai::Provider::Anthropic => AiProviderPy::Anthropic,
            eggsec::ai::Provider::OpenAICompatible => AiProviderPy::OpenAICompatible,
        }
    }
}

#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginLanguagePy {
    Python,
    Ruby,
    Rust,
}

#[pymethods]
impl PluginLanguagePy {
    fn __repr__(&self) -> String {
        format!("PluginLanguage.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl PluginLanguagePy {
    fn as_str(&self) -> &str {
        match self {
            PluginLanguagePy::Python => "Python",
            PluginLanguagePy::Ruby => "Ruby",
            PluginLanguagePy::Rust => "Rust",
        }
    }

    fn from_engine(engine: eggsec::ai::PluginLanguage) -> Self {
        match engine {
            eggsec::ai::PluginLanguage::Python => PluginLanguagePy::Python,
            eggsec::ai::PluginLanguage::Ruby => PluginLanguagePy::Ruby,
            eggsec::ai::PluginLanguage::Rust => PluginLanguagePy::Rust,
        }
    }
}

#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysisResultPy {
    #[pyo3(get)]
    pub reassessed_severity: String,
    #[pyo3(get)]
    pub exploitability: String,
    #[pyo3(get)]
    pub impact: String,
    remediation: Vec<String>,
    #[pyo3(get)]
    pub confidence: f32,
}

impl AiAnalysisResultPy {
    fn from_engine(engine: eggsec::ai::AiAnalysisResult) -> Self {
        Self {
            reassessed_severity: engine.reassessed_severity,
            exploitability: engine.exploitability,
            impact: engine.impact,
            remediation: engine.remediation,
            confidence: engine.confidence,
        }
    }
}

#[pymethods]
impl AiAnalysisResultPy {
    #[getter]
    fn remediation(&self) -> Vec<String> {
        self.remediation.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("reassessed_severity", &self.reassessed_severity)?;
        dict.set_item("exploitability", &self.exploitability)?;
        dict.set_item("impact", &self.impact)?;
        dict.set_item("remediation", &self.remediation)?;
        dict.set_item("confidence", self.confidence)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "AiAnalysisResult(severity={}, confidence={})",
            self.reassessed_severity, self.confidence
        )
    }
}

#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPayloadSuggestionPy {
    #[pyo3(get)]
    pub payload: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub expected_result: String,
}

#[pymethods]
impl AiPayloadSuggestionPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("payload", &self.payload)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("expected_result", &self.expected_result)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("AiPayloadSuggestion(payload={:.30}...)", self.payload)
    }
}

#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWafBypassSuggestionPy {
    #[pyo3(get)]
    pub technique: String,
    #[pyo3(get)]
    pub payload: String,
    #[pyo3(get)]
    pub explanation: String,
}

#[pymethods]
impl AiWafBypassSuggestionPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("technique", &self.technique)?;
        dict.set_item("payload", &self.payload)?;
        dict.set_item("explanation", &self.explanation)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("AiWafBypassSuggestion(technique={})", self.technique)
    }
}

#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCacheStatsPy {
    #[pyo3(get)]
    pub total_entries: usize,
    #[pyo3(get)]
    pub hit_count: u64,
    #[pyo3(get)]
    pub miss_count: u64,
    #[pyo3(get)]
    pub hit_rate: f64,
}

#[pymethods]
impl AiCacheStatsPy {
    fn __repr__(&self) -> String {
        format!(
            "AiCacheStats(entries={}, hit_rate={:.1}%)",
            self.total_entries,
            self.hit_rate * 100.0
        )
    }
}

#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadataPy {
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub author: String,
    #[pyo3(get)]
    pub version: String,
    tags: Vec<String>,
    #[pyo3(get)]
    pub ai_generated: bool,
}

impl ScriptMetadataPy {
    fn from_engine(engine: eggsec::ai::ScriptMetadata) -> Self {
        Self {
            description: engine.description,
            author: engine.author,
            version: engine.version,
            tags: engine.tags,
            ai_generated: engine.ai_generated,
        }
    }
}

#[pymethods]
impl ScriptMetadataPy {
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("description", &self.description)?;
        dict.set_item("author", &self.author)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("tags", &self.tags)?;
        dict.set_item("ai_generated", self.ai_generated)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("ScriptMetadata(author={})", self.author)
    }
}

#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedScriptPy {
    #[pyo3(get)]
    pub code: String,
    #[pyo3(get)]
    pub language: PluginLanguagePy,
    #[pyo3(get)]
    pub metadata: ScriptMetadataPy,
}

impl GeneratedScriptPy {
    fn from_engine(engine: eggsec::ai::GeneratedScript) -> Self {
        Self {
            code: engine.code,
            language: PluginLanguagePy::from_engine(engine.language),
            metadata: ScriptMetadataPy::from_engine(engine.metadata),
        }
    }
}

#[pymethods]
impl GeneratedScriptPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("code", &self.code)?;
        dict.set_item("language", self.language.as_str())?;
        dict.set_item("metadata", self.metadata.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "GeneratedScript(lang={}, len={})",
            self.language.as_str(),
            self.code.len()
        )
    }
}

#[pyclass]
pub struct AiCachePy {
    max_entries: usize,
    ttl_secs: u64,
}

#[pymethods]
impl AiCachePy {
    #[new]
    #[pyo3(signature = (max_entries=100, ttl_secs=1800))]
    fn new(max_entries: usize, ttl_secs: u64) -> Self {
        Self {
            max_entries,
            ttl_secs,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "AiCache(max_entries={}, ttl={})",
            self.max_entries, self.ttl_secs
        )
    }
}

fn build_chat_body(
    model: &str,
    prompt: &str,
    max_tokens: Option<u32>,
    temperature: f64,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens.unwrap_or(2048),
        "temperature": temperature,
    })
}

fn parse_analysis_response(response: &serde_json::Value) -> eggsec::ai::AiAnalysisResult {
    if let Some(obj) = response.as_object() {
        eggsec::ai::AiAnalysisResult {
            reassessed_severity: obj
                .get("reassessed_severity")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            exploitability: obj
                .get("exploitability")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            impact: obj
                .get("impact")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            remediation: obj
                .get("remediation")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            confidence: obj
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32,
        }
    } else {
        let text = response.as_str().unwrap_or("No response");
        eggsec::ai::AiAnalysisResult {
            reassessed_severity: "Unknown".to_string(),
            exploitability: text.to_string(),
            impact: "Unable to parse".to_string(),
            remediation: vec![text.to_string()],
            confidence: 0.5,
        }
    }
}

#[pyfunction]
#[pyo3(signature = (finding_json, api_key, provider="openai".to_string(), model=None))]
pub fn ai_analyze_finding(
    finding_json: String,
    api_key: String,
    provider: String,
    model: Option<String>,
) -> PyResult<AiAnalysisResultPy> {
    Python::with_gil(|py| {
        runtime_sync::block_on(py, async move {
            let provider_enum = eggsec::ai::Provider::from_str(&provider);
            let resolved_model = model.unwrap_or_else(|| provider_enum.default_model().to_string());
            let config = eggsec::config::AiConfig {
                provider,
                api_key: Some(eggsec_core::types::SensitiveString::new(&api_key)),
                model: Some(resolved_model.clone()),
                base_url: None,
                max_tokens: Some(1024),
                temperature: Some(0.3),
                max_payloads: 50,
                max_bypasses: 10,
            };

            let prompt = format!(
                "Analyze this security finding and provide: reassessed severity, \
                 exploitability assessment, impact analysis, remediation steps, \
                 and your confidence level (0.0-1.0).\n\nFinding: {}",
                finding_json
            );

            let body = build_chat_body(&resolved_model, &prompt, Some(1024), 0.3);

            match eggsec::ai::AiClient::new(config) {
                Ok(client) => match client.chat_completion_from_messages(&body).await {
                    Ok(response) => Ok(AiAnalysisResultPy::from_engine(parse_analysis_response(
                        &response,
                    ))),
                    Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "AI analysis failed: {}",
                        e
                    ))),
                },
                Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "AI client creation failed: {}",
                    e
                ))),
            }
        })
    })
}

#[pyfunction]
#[pyo3(signature = (finding_json, api_key, provider="openai".to_string(), model=None))]
pub fn async_ai_analyze_finding(
    finding_json: String,
    api_key: String,
    provider: String,
    model: Option<String>,
) -> PyResult<crate::runtime_async::PyFuture> {
    crate::runtime_async::spawn_async(async move {
        let provider_enum = eggsec::ai::Provider::from_str(&provider);
        let resolved_model = model.unwrap_or_else(|| provider_enum.default_model().to_string());
        let config = eggsec::config::AiConfig {
            provider,

            api_key: Some(eggsec_core::types::SensitiveString::new(&api_key)),
            model: Some(resolved_model.clone()),
            base_url: None,
            max_tokens: Some(1024),
            temperature: Some(0.3),
            max_payloads: 50,
            max_bypasses: 10,
        };

        let prompt = format!(
            "Analyze this security finding and provide: reassessed severity, \
             exploitability assessment, impact analysis, remediation steps, \
             and your confidence level (0.0-1.0).\n\nFinding: {}",
            finding_json
        );

        let body = build_chat_body(&resolved_model, &prompt, Some(1024), 0.3);

        match eggsec::ai::AiClient::new(config) {
            Ok(client) => match client.chat_completion_from_messages(&body).await {
                Ok(response) => Ok(AiAnalysisResultPy::from_engine(parse_analysis_response(
                    &response,
                ))),
                Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "AI analysis failed: {}",
                    e
                ))),
            },
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "AI client creation failed: {}",
                e
            ))),
        }
    })
}

#[pyfunction]
#[pyo3(signature = (vulnerability_type, target_context, api_key, provider="openai".to_string()))]
pub fn ai_generate_payloads(
    vulnerability_type: String,
    target_context: String,
    api_key: String,
    provider: String,
) -> PyResult<Vec<AiPayloadSuggestionPy>> {
    Python::with_gil(|py| {
        runtime_sync::block_on(py, async move {
            let provider_enum = eggsec::ai::Provider::from_str(&provider);
            let resolved_model = provider_enum.default_model().to_string();
            let config = eggsec::config::AiConfig {
                provider,
                api_key: Some(eggsec_core::types::SensitiveString::new(&api_key)),
                model: Some(resolved_model),
                base_url: None,
                max_tokens: Some(2048),
                temperature: Some(0.7),
                max_payloads: 50,
                max_bypasses: 10,
            };

            match eggsec::ai::AiClient::new(config) {
                Ok(client) => match client
                    .suggest_payloads(&vulnerability_type, &target_context)
                    .await
                {
                    Ok(payloads) => Ok(payloads
                        .into_iter()
                        .map(|p| AiPayloadSuggestionPy {
                            payload: p,
                            description: format!("AI-generated {} payload", vulnerability_type),
                            expected_result: "Verify response".to_string(),
                        })
                        .collect()),
                    Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "AI payload generation failed: {}",
                        e
                    ))),
                },
                Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "AI client creation failed: {}",
                    e
                ))),
            }
        })
    })
}

#[pyfunction]
#[pyo3(signature = (waf_name, blocked_payload, api_key, provider="openai".to_string()))]
pub fn ai_suggest_waf_bypass(
    waf_name: String,
    blocked_payload: String,
    api_key: String,
    provider: String,
) -> PyResult<Vec<AiWafBypassSuggestionPy>> {
    Python::with_gil(|py| {
        runtime_sync::block_on(py, async move {
            let provider_enum = eggsec::ai::Provider::from_str(&provider);
            let resolved_model = provider_enum.default_model().to_string();
            let config = eggsec::config::AiConfig {
                provider,
                api_key: Some(eggsec_core::types::SensitiveString::new(&api_key)),
                model: Some(resolved_model),
                base_url: None,
                max_tokens: Some(2048),
                temperature: Some(0.8),
                max_payloads: 50,
                max_bypasses: 10,
            };

            match eggsec::ai::AiClient::new(config) {
                Ok(client) => match client.suggest_waf_bypass(&waf_name, &blocked_payload).await {
                    Ok(suggestions) => Ok(suggestions
                        .into_iter()
                        .map(|s| AiWafBypassSuggestionPy {
                            technique: "AI-suggested bypass".to_string(),
                            payload: s,
                            explanation: format!("Bypass technique for {} WAF", waf_name),
                        })
                        .collect()),
                    Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "AI WAF bypass suggestion failed: {}",
                        e
                    ))),
                },
                Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "AI client creation failed: {}",
                    e
                ))),
            }
        })
    })
}
