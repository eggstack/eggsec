use crate::error::Result;
use crate::supply_chain::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TyposquatReport {
    pub packages_checked: usize,
    pub suspicious_packages: Vec<TyposquatFinding>,
    pub risk_level: TyposquatRiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TyposquatFinding {
    pub package_name: String,
    pub suspected_target: String,
    pub similarity_score: f64,
    pub techniques: Vec<TyposquatTechnique>,
    pub severity: Severity,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TyposquatTechnique {
    CharacterSwap,
    CharacterOmission,
    CharacterInsertion,
    CharacterReplacement,
    Hyphenation,
    Subdomain,
    Combosquatting,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TyposquatRiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

const WELL_KNOWN_PACKAGES: &[&str] = &[
    "requests",
    "flask",
    "django",
    "numpy",
    "pandas",
    "scipy",
    "tensorflow",
    "pytorch",
    "express",
    "lodash",
    "react",
    "angular",
    "vue",
    "axios",
    "webpack",
    "babel",
    "serde",
    "tokio",
    "actix",
    "rocket",
    "clap",
    "rand",
    "regex",
    "reqwest",
    "rails",
    "sinatra",
    "devise",
    "rspec",
    "sidekiq",
    "puma",
    "unicorn",
    "spring-boot",
    "hibernate",
    "jackson",
    "guava",
    "lombok",
    "log4j",
    "moment",
    "underscore",
    "async",
    "await",
    "chalk",
];

pub struct TyposquatDetector {
    threshold: f64,
}

impl TyposquatDetector {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    pub fn check_packages(&self, package_names: &[String]) -> Result<TyposquatReport> {
        let mut suspicious = Vec::new();

        for pkg in package_names {
            if let Some(finding) = self.check_package(pkg) {
                suspicious.push(finding);
            }
        }

        let risk_level = Self::calculate_risk(&suspicious);

        Ok(TyposquatReport {
            packages_checked: package_names.len(),
            suspicious_packages: suspicious,
            risk_level,
        })
    }

    pub fn check_package(&self, package_name: &str) -> Option<TyposquatFinding> {
        let lower = package_name.to_lowercase();

        for known in WELL_KNOWN_PACKAGES {
            let known_lower = known.to_lowercase();

            if lower == known_lower {
                continue;
            }

            let distance = Self::levenshtein_distance(&lower, &known_lower);
            let max_len = lower.len().max(known_lower.len()) as f64;
            if max_len == 0.0 {
                continue;
            }
            let similarity = 1.0 - (distance as f64 / max_len);

            if similarity >= self.threshold && similarity < 1.0 {
                let techniques = Self::detect_techniques(&lower, &known_lower);
                let severity = if similarity >= 0.9 {
                    Severity::Critical
                } else if similarity >= 0.8 {
                    Severity::High
                } else if similarity >= 0.7 {
                    Severity::Medium
                } else {
                    Severity::Low
                };

                return Some(TyposquatFinding {
                    package_name: package_name.to_string(),
                    suspected_target: known.to_string(),
                    similarity_score: similarity,
                    techniques,
                    severity,
                    recommendation: format!(
                        "Package '{}' is suspiciously similar to '{}'. Verify the package source.",
                        package_name, known
                    ),
                });
            }
        }

        None
    }

    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

        #[allow(clippy::needless_range_loop)]
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        #[allow(clippy::needless_range_loop)]
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    fn detect_techniques(s1: &str, s2: &str) -> Vec<TyposquatTechnique> {
        let mut techniques = Vec::new();

        if s1.len() == s2.len() {
            let diff_count = s1.chars().zip(s2.chars()).filter(|(a, b)| a != b).count();
            if diff_count == 1 {
                techniques.push(TyposquatTechnique::CharacterReplacement);
            } else if diff_count == 2 {
                let diffs: Vec<_> = s1.chars().zip(s2.chars()).filter(|(a, b)| a != b).collect();
                if diffs.len() == 2 && diffs[0].0 == diffs[1].1 && diffs[0].1 == diffs[1].0 {
                    techniques.push(TyposquatTechnique::CharacterSwap);
                } else {
                    techniques.push(TyposquatTechnique::CharacterReplacement);
                }
            }
        }

        if s1.len() == s2.len() + 1 {
            techniques.push(TyposquatTechnique::CharacterInsertion);
        }
        if s1.len() + 1 == s2.len() {
            techniques.push(TyposquatTechnique::CharacterOmission);
        }

        if s1.contains('-') && !s2.contains('-') {
            techniques.push(TyposquatTechnique::Hyphenation);
        }

        if s1.contains('.') && !s2.contains('.') {
            techniques.push(TyposquatTechnique::Subdomain);
        }

        if techniques.is_empty() {
            techniques.push(TyposquatTechnique::Combosquatting);
        }

        techniques
    }

    fn calculate_risk(findings: &[TyposquatFinding]) -> TyposquatRiskLevel {
        if findings.iter().any(|f| f.severity == Severity::Critical) {
            TyposquatRiskLevel::Critical
        } else if findings.iter().any(|f| f.severity == Severity::High) {
            TyposquatRiskLevel::High
        } else if findings.iter().any(|f| f.severity == Severity::Medium) {
            TyposquatRiskLevel::Medium
        } else if !findings.is_empty() {
            TyposquatRiskLevel::Low
        } else {
            TyposquatRiskLevel::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typosquat_detector_creation() {
        let detector = TyposquatDetector::new(0.7);
        assert_eq!(detector.threshold, 0.7);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(
            TyposquatDetector::levenshtein_distance("kitten", "sitting"),
            3
        );
        assert_eq!(TyposquatDetector::levenshtein_distance("", "abc"), 3);
        assert_eq!(TyposquatDetector::levenshtein_distance("abc", ""), 3);
        assert_eq!(TyposquatDetector::levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_detect_typosquat_character_swap() {
        let detector = TyposquatDetector::new(0.7);
        let finding = detector.check_package("reqeusts").unwrap();
        assert_eq!(finding.suspected_target, "requests");
        assert!(finding
            .techniques
            .iter()
            .any(|t| matches!(t, TyposquatTechnique::CharacterSwap)));
    }

    #[test]
    fn test_detect_typosquat_omission() {
        let detector = TyposquatDetector::new(0.7);
        let finding = detector.check_package("reqests").unwrap();
        assert_eq!(finding.suspected_target, "requests");
    }

    #[test]
    fn test_detect_typosquat_replacement() {
        let detector = TyposquatDetector::new(0.7);
        let finding = detector.check_package("requets").unwrap();
        assert_eq!(finding.suspected_target, "requests");
    }

    #[test]
    fn test_no_false_positive() {
        let detector = TyposquatDetector::new(0.7);
        let finding = detector.check_package("xyznonexistent123");
        assert!(finding.is_none());
    }

    #[test]
    fn test_check_multiple_packages() {
        let detector = TyposquatDetector::new(0.7);
        let packages = vec![
            "requests".to_string(),
            "reqeusts".to_string(),
            "flask".to_string(),
            "flsak".to_string(),
        ];
        let report = detector.check_packages(&packages).unwrap();
        assert_eq!(report.packages_checked, 4);
        assert_eq!(report.suspicious_packages.len(), 2);
    }

    #[test]
    fn test_risk_level_calculation() {
        let findings = vec![TyposquatFinding {
            package_name: "test".to_string(),
            suspected_target: "test".to_string(),
            similarity_score: 0.9,
            techniques: vec![TyposquatTechnique::CharacterReplacement],
            severity: Severity::Critical,
            recommendation: "Test".to_string(),
        }];
        assert_eq!(
            TyposquatDetector::calculate_risk(&findings),
            TyposquatRiskLevel::Critical
        );
    }

    #[test]
    fn test_technique_detection_swap() {
        let techniques = TyposquatDetector::detect_techniques("ab", "ba");
        assert!(techniques
            .iter()
            .any(|t| matches!(t, TyposquatTechnique::CharacterSwap)));
    }

    #[test]
    fn test_technique_detection_insertion() {
        let techniques = TyposquatDetector::detect_techniques("abc", "ab");
        assert!(techniques
            .iter()
            .any(|t| matches!(t, TyposquatTechnique::CharacterInsertion)));
    }

    #[test]
    fn test_technique_detection_omission() {
        let techniques = TyposquatDetector::detect_techniques("ab", "abc");
        assert!(techniques
            .iter()
            .any(|t| matches!(t, TyposquatTechnique::CharacterOmission)));
    }
}
