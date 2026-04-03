use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IamAnalysisResult {
    pub account_id: Option<String>,
    pub privilege_escalation_paths: Vec<PrivilegeEscalationPath>,
    pub risky_policies: Vec<RiskyPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegeEscalationPath {
    pub path_id: String,
    pub severity: Severity,
    pub description: String,
    pub required_permissions: Vec<String>,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskyPolicy {
    pub policy_name: String,
    pub risk_type: String,
    pub severity: Severity,
    pub description: String,
}

pub use crate::types::Severity;

pub const KNOWN_ESCALATION_PATTERNS: &[(&str, &str, &[&str], &str)] = &[
    (
        "iam:PassRole + lambda:CreateFunction",
        "Create Lambda with elevated role",
        &[
            "iam:PassRole",
            "lambda:CreateFunction",
            "lambda:InvokeFunction",
        ],
        "Restrict iam:PassRole to specific roles only",
    ),
    (
        "iam:CreateAccessKey",
        "Create access keys for other users",
        &["iam:CreateAccessKey"],
        "Deny iam:CreateAccessKey except for self",
    ),
    (
        "iam:AttachUserPolicy",
        "Attach admin policy to user",
        &["iam:AttachUserPolicy", "iam:CreatePolicyVersion"],
        "Restrict policy attachment to specific ARNs",
    ),
    (
        "iam:PutUserPolicy",
        "Add inline admin policy to user",
        &["iam:PutUserPolicy"],
        "Deny iam:PutUserPolicy or restrict with conditions",
    ),
    (
        "iam:CreatePolicyVersion",
        "Escalate via policy version bypass",
        &["iam:CreatePolicyVersion", "iam:SetDefaultPolicyVersion"],
        "Limit number of policy versions and require approval",
    ),
    (
        "iam:AddUserToGroup",
        "Add self to admin group",
        &["iam:AddUserToGroup"],
        "Restrict group membership changes",
    ),
    (
        "iam:UpdateLoginProfile",
        "Reset another user's password",
        &["iam:UpdateLoginProfile"],
        "Deny iam:UpdateLoginProfile for other users",
    ),
    (
        "sts:AssumeRole",
        "Assume role with higher privileges",
        &["sts:AssumeRole"],
        "Restrict AssumeRole to specific trusted roles",
    ),
    (
        "glue:UpdateDevEndpoint",
        "Escalate via Glue Dev Endpoint",
        &["glue:UpdateDevEndpoint", "glue:CreateDevEndpoint"],
        "Restrict Glue DevEndpoint creation and updates",
    ),
    (
        "lambda:UpdateFunctionCode",
        "Inject code into existing Lambda",
        &["lambda:UpdateFunctionCode", "lambda:InvokeFunction"],
        "Restrict Lambda code updates to CI/CD pipeline roles",
    ),
    (
        "cloudformation:CreateStack",
        "Create stack with admin resources",
        &["cloudformation:CreateStack", "iam:PassRole"],
        "Restrict CloudFormation to approved templates",
    ),
    (
        "datapipeline:CreatePipeline + datapipeline:PutPipelineDefinition",
        "Escalate via Data Pipeline",
        &[
            "datapipeline:CreatePipeline",
            "datapipeline:PutPipelineDefinition",
        ],
        "Restrict Data Pipeline creation",
    ),
];

pub struct IamAnalyzer;

impl Default for IamAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl IamAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_policy(&self, policy_document: &str) -> Vec<RiskyPolicy> {
        let mut risks = Vec::new();
        let lower = policy_document.to_lowercase();

        if lower.contains("\"action\": \"*\"") || lower.contains("\"action\": [\"*\"]") {
            risks.push(RiskyPolicy {
                policy_name: "Wildcard Action Policy".to_string(),
                risk_type: "Overly Permissive".to_string(),
                severity: Severity::Critical,
                description: "Policy grants all actions (*)".to_string(),
            });
        }

        if lower.contains("\"resource\": \"*\"") || lower.contains("\"resource\": [\"*\"]") {
            risks.push(RiskyPolicy {
                policy_name: "Wildcard Resource Policy".to_string(),
                risk_type: "Overly Permissive".to_string(),
                severity: Severity::High,
                description: "Policy applies to all resources (*)".to_string(),
            });
        }

        for (pattern_id, description, permissions, _mitigation) in KNOWN_ESCALATION_PATTERNS {
            let all_present = permissions
                .iter()
                .all(|p| lower.contains(&p.to_lowercase()));
            if all_present {
                risks.push(RiskyPolicy {
                    policy_name: format!("Escalation: {}", pattern_id),
                    risk_type: "Privilege Escalation".to_string(),
                    severity: Severity::Critical,
                    description: description.to_string(),
                });
            }
        }

        risks
    }

    pub fn get_escalation_patterns(&self) -> Vec<PrivilegeEscalationPath> {
        KNOWN_ESCALATION_PATTERNS
            .iter()
            .map(
                |(path_id, description, permissions, mitigation)| PrivilegeEscalationPath {
                    path_id: path_id.to_string(),
                    severity: Severity::High,
                    description: description.to_string(),
                    required_permissions: permissions.iter().map(|s| s.to_string()).collect(),
                    mitigation: mitigation.to_string(),
                },
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iam_analyzer_creation() {
        let analyzer = IamAnalyzer::new();
        let patterns = analyzer.get_escalation_patterns();
        assert!(patterns.len() >= 12);
    }

    #[test]
    fn test_known_patterns_count() {
        assert!(KNOWN_ESCALATION_PATTERNS.len() >= 12);
    }

    #[test]
    fn test_analyze_wildcard_action() {
        let analyzer = IamAnalyzer::new();
        let policy = r#"{"Version": "2012-10-17", "Statement": [{"Effect": "Allow", "Action": "*", "Resource": "*"}]}"#;
        let risks = analyzer.analyze_policy(policy);
        assert!(!risks.is_empty());
        assert!(risks.iter().any(|r| r.risk_type == "Overly Permissive"));
    }

    #[test]
    fn test_analyze_safe_policy() {
        let analyzer = IamAnalyzer::new();
        let policy = r#"{"Version": "2012-10-17", "Statement": [{"Effect": "Allow", "Action": "s3:GetObject", "Resource": "arn:aws:s3:::my-bucket/*"}]}"#;
        let risks = analyzer.analyze_policy(policy);
        assert!(risks.is_empty());
    }

    #[test]
    fn test_escalation_pattern_detection() {
        let analyzer = IamAnalyzer::new();
        let policy = r#"{"Statement": [{"Effect": "Allow", "Action": ["iam:CreateAccessKey"], "Resource": "*"}]}"#;
        let risks = analyzer.analyze_policy(policy);
        assert!(risks.iter().any(|r| r.risk_type == "Privilege Escalation"));
    }
}
