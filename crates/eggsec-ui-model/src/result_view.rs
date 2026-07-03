use serde::{Deserialize, Serialize};

use eggsec_runtime::event::{TaskOutcome, TaskResultEnvelope};

use super::artifact_view::ArtifactView;

/// Frontend-neutral result envelope view for rendering task outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultEnvelopeView {
    pub kind: String,
    pub kind_label: String,
    pub summary: Option<String>,
    pub payload: serde_json::Value,
    pub artifacts: Vec<ArtifactView>,
    pub artifact_count: usize,
    pub supports_rich_tui: bool,
    pub supports_json_detail: bool,
}

impl From<&TaskResultEnvelope> for ResultEnvelopeView {
    fn from(env: &TaskResultEnvelope) -> Self {
        let artifacts: Vec<_> = env.artifacts.iter().map(ArtifactView::from).collect();
        let renderer = super::renderer_registry::renderer_for_kind(&env.kind);
        Self {
            kind: env.kind.clone(),
            kind_label: renderer.map_or("Unknown", |r| r.title).into(),
            summary: env.summary.clone(),
            payload: env.payload.clone(),
            artifact_count: artifacts.len(),
            artifacts,
            supports_rich_tui: renderer.map_or(false, |r| r.supports_rich_tui),
            supports_json_detail: renderer.map_or(true, |r| r.supports_json_detail),
        }
    }
}

/// Frontend-neutral view of a task outcome (any variant).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeView {
    pub outcome_type: String,
    pub summary: Option<String>,
    pub envelope: Option<ResultEnvelopeView>,
    pub text_content: Option<String>,
    pub artifact_ref: Option<ArtifactView>,
}

impl From<&TaskOutcome> for OutcomeView {
    fn from(o: &TaskOutcome) -> Self {
        match o {
            TaskOutcome::Result(env) => Self {
                outcome_type: "result".into(),
                summary: env.summary.clone(),
                envelope: Some(ResultEnvelopeView::from(env)),
                text_content: None,
                artifact_ref: None,
            },
            TaskOutcome::Text(text) => Self {
                outcome_type: "text".into(),
                summary: Some(text.clone()),
                envelope: None,
                text_content: Some(text.clone()),
                artifact_ref: None,
            },
            TaskOutcome::Json(val) => Self {
                outcome_type: "json".into(),
                summary: val
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .map(String::from),
                envelope: None,
                text_content: None,
                artifact_ref: None,
            },
            TaskOutcome::Artifact {
                artifact_id,
                summary,
            } => Self {
                outcome_type: "artifact".into(),
                summary: summary.clone(),
                envelope: None,
                text_content: None,
                artifact_ref: Some(ArtifactView {
                    id: artifact_id.clone(),
                    kind: "artifact".into(),
                    path: None,
                    mime_type: None,
                    summary: summary.clone(),
                }),
            },
            TaskOutcome::Empty => Self {
                outcome_type: "empty".into(),
                summary: None,
                envelope: None,
                text_content: None,
                artifact_ref: None,
            },
        }
    }
}
