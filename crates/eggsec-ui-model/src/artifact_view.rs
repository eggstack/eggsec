use serde::{Deserialize, Serialize};

use eggsec_runtime::event::ArtifactRef;

/// Frontend-neutral artifact view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactView {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub mime_type: Option<String>,
    pub summary: Option<String>,
}

impl From<&ArtifactRef> for ArtifactView {
    fn from(a: &ArtifactRef) -> Self {
        Self {
            id: a.id.clone(),
            kind: a.kind.clone(),
            path: a.path.clone(),
            mime_type: a.mime_type.clone(),
            summary: a.summary.clone(),
        }
    }
}
