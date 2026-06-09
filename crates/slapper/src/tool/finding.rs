//! Finding types with engine-specific `From` implementations.
//!
//! The core `Finding`, `FindingType`, and `ResponseSeverity` types are defined
//! in `slapper-tool-core`. This module re-exports them and adds `From` impls
//! that depend on scanner/fuzzer/recon types from the main `slapper` crate.

pub use slapper_tool_core::finding::{Finding, FindingType, ResponseSeverity};

use rustc_hash::FxHashMap;

impl From<crate::fuzzer::FuzzResult> for Finding {
    fn from(result: crate::fuzzer::FuzzResult) -> Self {
        let severity = ResponseSeverity::from(result.detected_severity);
        let description = if result.leaks_found.is_empty() {
            String::new()
        } else {
            result.leaks_found.join(", ")
        };
        let location = format!(
            "{} - {}",
            result.payload.payload_type, result.payload.payload
        );
        let mut metadata = FxHashMap::default();
        metadata.insert(
            "status_code".to_string(),
            serde_json::Value::Number(result.status_code.into()),
        );
        metadata.insert(
            "response_time_ms".to_string(),
            serde_json::Value::Number(result.response_time_ms.into()),
        );
        metadata.insert(
            "is_waf_blocked".to_string(),
            serde_json::Value::Bool(result.is_waf_blocked),
        );
        metadata.insert(
            "is_anomaly".to_string(),
            serde_json::Value::Bool(result.is_anomaly),
        );
        metadata.insert(
            "payload".to_string(),
            serde_json::to_value(&result.payload)
                .inspect_err(|e| {
                    tracing::debug!(error = %e, "Failed to serialize payload metadata");
                })
                .unwrap_or_default(),
        );

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: result.payload.description,
            description,
            location,
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata,
        }
    }
}

impl From<crate::scanner::ports::PortResult> for Finding {
    fn from(result: crate::scanner::ports::PortResult) -> Self {
        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::OpenPort,
            severity: ResponseSeverity::Info,
            title: format!("Open port: {}/tcp ({})", result.port, result.service),
            description: format!(
                "Port {} is open running service: {}",
                result.port, result.service
            ),
            location: format!("{}:{}", result.port, result.service),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "port".to_string(),
                    serde_json::Value::Number(result.port.into()),
                );
                m.insert(
                    "service".to_string(),
                    serde_json::Value::String(result.service),
                );
                m.insert(
                    "status".to_string(),
                    serde_json::Value::String(result.status),
                );
                m
            },
        }
    }
}

impl From<crate::scanner::fingerprint::ServiceFingerprint> for Finding {
    fn from(fp: crate::scanner::fingerprint::ServiceFingerprint) -> Self {
        let service_info = fp
            .product
            .as_ref()
            .or(fp.version.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let banner_snippet = fp.banner.as_ref().map(|b| {
            let trimmed = b.chars().take(200).collect::<String>();
            if b.len() > 200 {
                format!("{}...", trimmed)
            } else {
                trimmed
            }
        });

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Service,
            severity: ResponseSeverity::Info,
            title: format!("Service detected: {} on port {}", service_info, fp.port),
            description: format!(
                "Detected {} (confidence: {}){}",
                service_info,
                fp.confidence,
                fp.version
                    .as_ref()
                    .map(|v| format!(" version {}", v))
                    .unwrap_or_default()
            ),
            location: format!("port {}", fp.port),
            evidence: banner_snippet,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "port".to_string(),
                    serde_json::Value::Number(fp.port.into()),
                );
                m.insert(
                    "service".to_string(),
                    serde_json::Value::String(fp.service.clone()),
                );
                m.insert(
                    "product".to_string(),
                    serde_json::to_value(&fp.product)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize product metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "version".to_string(),
                    serde_json::to_value(&fp.version)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize version metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "confidence".to_string(),
                    serde_json::Value::Number(fp.confidence.into()),
                );
                m.insert(
                    "banner".to_string(),
                    serde_json::to_value(&fp.banner)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize banner metadata");
                        })
                        .unwrap_or_default(),
                );
                m
            },
        }
    }
}

impl From<crate::scanner::udp_fingerprint::UdpServiceFingerprint> for Finding {
    fn from(fp: crate::scanner::udp_fingerprint::UdpServiceFingerprint) -> Self {
        let banner_snippet = fp.banner.as_ref().map(|b| {
            let trimmed = b.chars().take(200).collect::<String>();
            if b.len() > 200 {
                format!("{}...", trimmed)
            } else {
                trimmed
            }
        });

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Service,
            severity: ResponseSeverity::Info,
            title: format!("UDP service detected: {} on port {}", fp.service, fp.port),
            description: format!("Detected {} (confidence: {})", fp.service, fp.confidence),
            location: format!("port {}", fp.port),
            evidence: banner_snippet,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "port".to_string(),
                    serde_json::Value::Number(fp.port.into()),
                );
                m.insert("service".to_string(), serde_json::Value::String(fp.service));
                m.insert(
                    "response".to_string(),
                    serde_json::to_value(&fp.response)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize response metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "banner".to_string(),
                    serde_json::to_value(&fp.banner)
                        .inspect_err(|e| {
                            tracing::debug!(error = %e, "Failed to serialize banner metadata");
                        })
                        .unwrap_or_default(),
                );
                m.insert(
                    "confidence".to_string(),
                    serde_json::Value::Number(fp.confidence.into()),
                );
                m
            },
        }
    }
}

impl From<crate::scanner::endpoints::EndpointResult> for Finding {
    fn from(result: crate::scanner::endpoints::EndpointResult) -> Self {
        let severity = if result.interesting {
            ResponseSeverity::Low
        } else {
            ResponseSeverity::Info
        };
        let title = if result.interesting {
            format!(
                "Interesting endpoint: {} ({})",
                result.path, result.status_code
            )
        } else {
            format!(
                "Endpoint discovered: {} ({})",
                result.path, result.status_code
            )
        };

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Endpoint,
            severity,
            title,
            description: format!(
                "Path: {}, Status: {} ({}){}",
                result.path,
                result.status_code,
                result.status_text,
                result
                    .content_length
                    .map(|l| format!(", {} bytes", l))
                    .unwrap_or_default()
            ),
            location: result.path.clone(),
            evidence: None,
            cve_ids: vec![],
            remediation: None,
            references: vec![],
            metadata: {
                let mut m = FxHashMap::default();
                m.insert("path".to_string(), serde_json::Value::String(result.path));
                m.insert(
                    "status_code".to_string(),
                    serde_json::Value::Number(result.status_code.into()),
                );
                m.insert(
                    "status_text".to_string(),
                    serde_json::Value::String(result.status_text),
                );
                m.insert(
                    "content_length".to_string(),
                    serde_json::to_value(result.content_length).inspect_err(|e| {
                        tracing::debug!(error = %e, "Failed to serialize content_length metadata");
                    }).unwrap_or_default(),
                );
                m.insert(
                    "response_time_ms".to_string(),
                    serde_json::Value::Number(result.response_time_ms.into()),
                );
                m.insert(
                    "interesting".to_string(),
                    serde_json::Value::Bool(result.interesting),
                );
                m
            },
        }
    }
}

impl From<crate::recon::cve::VulnerabilityInfo> for Finding {
    fn from(v: crate::recon::cve::VulnerabilityInfo) -> Self {
        let severity = match v.severity.to_lowercase().as_str() {
            "critical" => ResponseSeverity::Critical,
            "high" => ResponseSeverity::High,
            "medium" | "moderate" => ResponseSeverity::Medium,
            "low" => ResponseSeverity::Low,
            _ => ResponseSeverity::Info,
        };

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            finding_type: FindingType::Vulnerability,
            severity,
            title: format!(
                "{}: {}",
                v.cve_id,
                v.description.split('.').next().unwrap_or(&v.description)
            ),
            description: v.description.clone(),
            location: v.affected_product.clone(),
            evidence: None,
            cve_ids: vec![v.cve_id.clone()],
            remediation: None,
            references: v.references.clone(),
            metadata: {
                let mut m = FxHashMap::default();
                m.insert(
                    "cvss_score".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(v.cvss_score as f64)
                            .unwrap_or(serde_json::Number::from(0)),
                    ),
                );
                m.insert(
                    "severity".to_string(),
                    serde_json::Value::String(v.severity),
                );
                m.insert(
                    "affected_product".to_string(),
                    serde_json::Value::String(v.affected_product),
                );
                m.insert(
                    "published_date".to_string(),
                    serde_json::to_value(&v.published_date).inspect_err(|e| {
                        tracing::debug!(error = %e, "Failed to serialize published_date metadata");
                    }).unwrap_or_default(),
                );
                m
            },
        }
    }
}
