use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCriticality {
    pub asset_id: String,
    pub technology_score: f32,
    pub environment_score: f32,
    pub data_sensitivity: f32,
    pub user_base: f32,
    pub overall_score: f32,
}

impl AssetCriticality {
    pub fn new(asset_id: &str) -> Self {
        Self {
            asset_id: asset_id.to_string(),
            technology_score: 5.0,
            environment_score: 5.0,
            data_sensitivity: 5.0,
            user_base: 5.0,
            overall_score: 5.0,
        }
    }

    pub fn with_technology(mut self, score: f32) -> Self {
        self.technology_score = Self::clamp_score(score);
        self.recalculate();
        self
    }

    pub fn with_environment(mut self, score: f32) -> Self {
        self.environment_score = Self::clamp_score(score);
        self.recalculate();
        self
    }

    pub fn with_data_sensitivity(mut self, score: f32) -> Self {
        self.data_sensitivity = Self::clamp_score(score);
        self.recalculate();
        self
    }

    pub fn with_user_base(mut self, score: f32) -> Self {
        self.user_base = Self::clamp_score(score);
        self.recalculate();
        self
    }

    fn clamp_score(score: f32) -> f32 {
        score.clamp(0.0, 10.0)
    }

    fn recalculate(&mut self) {
        self.technology_score = Self::clamp_score(self.technology_score);
        self.environment_score = Self::clamp_score(self.environment_score);
        self.data_sensitivity = Self::clamp_score(self.data_sensitivity);
        self.user_base = Self::clamp_score(self.user_base);
        self.overall_score = (self.technology_score * 0.3
            + self.environment_score * 0.25
            + self.data_sensitivity * 0.3
            + self.user_base * 0.15)
            .min(10.0);
    }
}

pub fn assess_asset(asset_id: &str, asset_type: &str) -> AssetCriticality {
    let base = AssetCriticality::new(asset_id);

    match asset_type {
        "database" => base.with_technology(9.0).with_data_sensitivity(10.0),
        "web_server" => base.with_technology(7.0).with_environment(8.0),
        "api" => base.with_technology(8.0).with_data_sensitivity(7.0),
        "workstation" => base.with_technology(4.0),
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_criticality() {
        let asset = assess_asset("asset-1", "database");
        assert!(asset.overall_score >= 7.0);
    }

    #[test]
    fn test_asset_builder() {
        let asset = AssetCriticality::new("test")
            .with_technology(10.0)
            .with_data_sensitivity(10.0);
        assert_eq!(asset.overall_score, 8.0);
    }

    #[test]
    fn test_asset_clamps_scores() {
        let asset = AssetCriticality::new("test")
            .with_technology(15.0)
            .with_data_sensitivity(-2.0);
        assert_eq!(asset.technology_score, 10.0);
        assert_eq!(asset.data_sensitivity, 0.0);
    }
}
