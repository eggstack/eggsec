use crate::cli::ai_analyze::AiAnalyzeArgs;
use crate::output::ai_schema::{AiEvidence, AiFinding, AiOutput, AiRemediation};
use crate::types::Severity;
use anyhow::{Context, Result};

pub async fn handle_ai_analyze(
    ctx: &crate::commands::CommandContext,
    mut args: AiAnalyzeArgs,
) -> Result<()> {
    args.json |= ctx.json;
    let config = &ctx.config;
    let ai_config = config.ai.clone().unwrap_or_default();

    if ai_config.api_key.is_none() {
        return Err(anyhow::anyhow!(
            "AI analysis requires an API key. Set AI_API_KEY in config or OPENAI_API_KEY environment variable."
        ));
    }

    let findings_data = if let Some(ref input_path) = args.input {
        let content = tokio::fs::read_to_string(input_path)
            .await
            .with_context(|| format!("Failed to read input file: {}", input_path))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .with_context(|| "Failed to parse findings JSON")?
    } else {
        let mut input = String::new();
        use tokio::io::AsyncBufReadExt;
        let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
        stdin
            .read_line(&mut input)
            .await
            .context("Failed to read from stdin")?;
        serde_json::from_str(&input).context("Failed to parse findings JSON from stdin")?
    };

    let findings_list = if findings_data.is_array() {
        findings_data.as_array().unwrap().clone()
    } else if let Some(findings) = findings_data.get("findings").and_then(|v| v.as_array()) {
        findings.clone()
    } else {
        vec![findings_data]
    };

    eprintln!("Analyzing {} finding(s) with AI...", findings_list.len());

    let client = crate::ai::AiClient::new(ai_config).context("Failed to initialize AI client")?;
    let ai_response = client
        .analyze_findings(&findings_list)
        .await
        .context("AI analysis failed")?;

    let ai_text = ai_response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("No analysis available returned by AI.");

    let ai_findings = parse_ai_analysis(ai_text, &findings_list, &args.analysis_type);

    let output = AiOutput::from_findings(ai_findings);

    let output_str = if args.json {
        serde_json::to_string_pretty(&output)?
    } else {
        format_ai_output(&output)
    };

    if let Some(ref output_path) = args.output {
        tokio::fs::write(output_path, &output_str)
            .await
            .with_context(|| format!("Failed to write output to: {}", output_path))?;
        eprintln!("AI analysis written to {}", output_path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

fn parse_ai_analysis(
    ai_text: &str,
    raw_findings: &[serde_json::Value],
    analysis_type: &str,
) -> Vec<AiFinding> {
    let mut findings = Vec::new();

    for raw in raw_findings {
        let severity = raw
            .get("severity")
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<Severity>().ok())
            .unwrap_or(Severity::Medium);

        let title = raw
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("Unknown finding")
            .to_string();

        let description = raw
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let mut evidence = Vec::new();
        if let Some(evidence_data) = raw.get("evidence") {
            if let Some(evidence_str) = evidence_data.as_str() {
                evidence.push(AiEvidence {
                    source: "scan".to_string(),
                    content: evidence_str.to_string(),
                    relevance: 0.8,
                });
            } else if let Some(arr) = evidence_data.as_array() {
                for ev in arr {
                    if let Some(s) = ev.as_str() {
                        evidence.push(AiEvidence {
                            source: "scan".to_string(),
                            content: s.to_string(),
                            relevance: 0.8,
                        });
                    }
                }
            }
        }

        let mut remediation = Vec::new();
        if analysis_type.contains("remediation") || analysis_type == "full" {
            remediation.push(AiRemediation {
                priority: match severity {
                    Severity::Critical => 1,
                    Severity::High => 2,
                    Severity::Medium => 3,
                    Severity::Low => 4,
                    Severity::Info => 5,
                },
                action: format!("Address {}: {}", severity.as_str().to_uppercase(), title),
                effort: if severity >= Severity::High {
                    "high"
                } else {
                    "medium"
                }
                .to_string(),
            });
        }

        if analysis_type.contains("attack-chain") || analysis_type == "full" {
            evidence.push(AiEvidence {
                source: "ai-analysis".to_string(),
                content: ai_text.chars().take(500).collect(),
                relevance: 0.6,
            });
        }

        findings.push(AiFinding {
            title,
            severity,
            description,
            evidence,
            remediation,
            confidence: 0.7,
        });
    }

    findings
}

fn format_ai_output(output: &AiOutput) -> String {
    let mut s = String::new();
    s.push_str("═══════════════════════════════════════════════════════\n");
    s.push_str("              AI Security Analysis Report\n");
    s.push_str("═══════════════════════════════════════════════════════\n\n");

    let summary = &output.summary;
    s.push_str(&format!("Total Findings: {}\n", summary.total_findings));
    s.push_str(&format!(
        "Critical: {} | High: {} | Medium: {} | Low: {} | Info: {}\n",
        summary.critical_count,
        summary.high_count,
        summary.medium_count,
        summary.low_count,
        summary.info_count
    ));
    s.push_str(&format!("Risk Score: {:.1}/10\n\n", summary.risk_score));

    if !summary.executive_summary.is_empty() {
        s.push_str("Executive Summary:\n");
        s.push_str(&format!("  {}\n\n", summary.executive_summary));
    }

    for (i, finding) in output.findings.iter().enumerate() {
        s.push_str(&format!("--- Finding #{} ---\n", i + 1));
        s.push_str(&format!(
            "[{}] {}\n",
            finding.severity.as_str().to_uppercase(),
            finding.title
        ));
        if !finding.description.is_empty() {
            s.push_str(&format!("  {}\n", finding.description));
        }
        s.push_str(&format!(
            "  Confidence: {:.0}%\n",
            finding.confidence * 100.0
        ));

        if !finding.evidence.is_empty() {
            s.push_str("  Evidence:\n");
            for ev in &finding.evidence {
                s.push_str(&format!(
                    "    - [{}] {}\n",
                    ev.source,
                    ev.content.chars().take(100).collect::<String>()
                ));
            }
        }

        if !finding.remediation.is_empty() {
            s.push_str("  Remediation:\n");
            for rem in &finding.remediation {
                s.push_str(&format!(
                    "    [P{}] {} (effort: {})\n",
                    rem.priority, rem.action, rem.effort
                ));
            }
        }
        s.push('\n');
    }

    s
}
