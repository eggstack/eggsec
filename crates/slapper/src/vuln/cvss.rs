use crate::error::Result;
use serde::{Deserialize, Serialize};

macro_rules! min {
    ($a:expr, $b:expr) => {
        if $a < $b {
            $a
        } else {
            $b
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvssScore {
    pub base_score: f32,
    pub temporal_score: f32,
    pub environmental_score: f32,
    pub vector: String,
}

impl CvssScore {
    pub fn from_vector(vector: &str) -> Result<Self> {
        let base_score = calculate_base_score_from_vector(vector);

        Ok(Self {
            base_score,
            temporal_score: base_score,
            environmental_score: base_score,
            vector: vector.to_string(),
        })
    }

    pub fn base_score(&self) -> f32 {
        self.base_score
    }

    pub fn severity(&self) -> &'static str {
        match self.base_score as u8 {
            0 => "NONE",
            1..=3 => "LOW",
            4..=6 => "MEDIUM",
            7..=8 => "HIGH",
            9..=10 => "CRITICAL",
            _ => "UNKNOWN",
        }
    }

    pub fn temporal_score(&self) -> f32 {
        self.temporal_score
    }

    #[allow(clippy::too_many_arguments)]
    pub fn calculate_base(
        attack_vector: &str,
        attack_complexity: &str,
        privileges_required: &str,
        user_interaction: &str,
        scope: &str,
        confidentiality: &str,
        integrity: &str,
        availability: &str,
    ) -> f32 {
        let av = match attack_vector {
            "N" => 0.85,
            "A" => 0.62,
            "L" => 0.55,
            "P" => 0.20,
            _ => 0.85,
        };

        let ac = match attack_complexity {
            "L" => 0.77,
            "H" => 0.44,
            _ => 0.77,
        };

        let pr_u = match privileges_required {
            "N" => 0.85,
            "L" => 0.62,
            "H" => 0.27,
            _ => 0.85,
        };

        let pr_c = if scope == "C" {
            match privileges_required {
                "N" => 0.85,
                "L" => 0.68,
                "H" => 0.50,
                _ => 0.85,
            }
        } else {
            pr_u
        };

        let ui = match user_interaction {
            "N" => 0.85,
            "R" => 0.62,
            _ => 0.85,
        };

        let c = match confidentiality {
            "H" => 0.56,
            "L" => 0.22,
            "N" => 0.0,
            _ => 0.0,
        };

        let i = match integrity {
            "H" => 0.56,
            "L" => 0.22,
            "N" => 0.0,
            _ => 0.0,
        };

        let a = match availability {
            "H" => 0.56,
            "L" => 0.22,
            "N" => 0.0,
            _ => 0.0,
        };

        let iss = 1.0 - ((1.0 - c) * (1.0 - i) * (1.0 - a));

        if scope == "U" {
            let impact = 6.42 * iss;
            let exploitability = 8.22 * av * ac * pr_u * ui;
            if impact == 0.0 {
                0.0
            } else {
                let score = min!(impact + exploitability, 10.0);
                f32::floor(score * 10.0) / 10.0
            }
        } else {
            let impact = 7.52 * (iss - 0.029) - 3.25 * (iss - 0.02).powf(15.0);
            let exploitability = 8.22 * av * ac * pr_c * ui;
            if impact == 0.0 {
                0.0
            } else {
                let score = min!(1.08 * (impact + exploitability), 10.0);
                f32::floor(score * 10.0) / 10.0
            }
        }
    }
}

fn calculate_base_score_from_vector(vector: &str) -> f32 {
    let mut av = 0.85;
    let mut ac = 0.77;
    let mut pr = 0.85;
    let mut ui = 0.85;
    let mut scope = "U";
    let mut c = 0.0;
    let mut i = 0.0;
    let mut a = 0.0;

    for part in vector.split('/') {
        let parts: Vec<&str> = part.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        match parts[0] {
            "AV" => {
                av = match parts[1] {
                    "N" => 0.85,
                    "A" => 0.62,
                    "L" => 0.55,
                    "P" => 0.20,
                    _ => 0.85,
                }
            }
            "AC" => {
                ac = match parts[1] {
                    "L" => 0.77,
                    "H" => 0.44,
                    _ => 0.77,
                }
            }
            "PR" => {
                pr = match parts[1] {
                    "N" => 0.85,
                    "L" => 0.62,
                    "H" => 0.27,
                    _ => 0.85,
                }
            }
            "UI" => {
                ui = match parts[1] {
                    "N" => 0.85,
                    "R" => 0.62,
                    _ => 0.85,
                }
            }
            "S" => scope = parts[1],
            "C" => {
                c = match parts[1] {
                    "H" => 0.56,
                    "L" => 0.22,
                    "N" => 0.0,
                    _ => 0.0,
                }
            }
            "I" => {
                i = match parts[1] {
                    "H" => 0.56,
                    "L" => 0.22,
                    "N" => 0.0,
                    _ => 0.0,
                }
            }
            "A" => {
                a = match parts[1] {
                    "H" => 0.56,
                    "L" => 0.22,
                    "N" => 0.0,
                    _ => 0.0,
                }
            }
            _ => {}
        }
    }

    CvssScore::calculate_base(
        if av == 0.85 {
            "N"
        } else if av == 0.62 {
            "A"
        } else if av == 0.55 {
            "L"
        } else {
            "P"
        },
        if ac == 0.77 { "L" } else { "H" },
        if pr == 0.85 {
            "N"
        } else if pr == 0.62 {
            "L"
        } else {
            "H"
        },
        if ui == 0.85 { "N" } else { "R" },
        scope,
        if c == 0.56 {
            "H"
        } else if c == 0.22 {
            "L"
        } else {
            "N"
        },
        if i == 0.56 {
            "H"
        } else if i == 0.22 {
            "L"
        } else {
            "N"
        },
        if a == 0.56 {
            "H"
        } else if a == 0.22 {
            "L"
        } else {
            "N"
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvss_score_from_vector() {
        let score = CvssScore::from_vector("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap();
        assert!(score.base_score >= 9.0);
    }

    #[test]
    fn test_base_score_calculation() {
        let score = CvssScore::calculate_base("N", "L", "N", "N", "U", "H", "H", "H");
        assert!(score >= 9.0);
    }
}
