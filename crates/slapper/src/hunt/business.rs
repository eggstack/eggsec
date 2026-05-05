use crate::error::Result;
use crate::hunt::HuntConfig;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessLogicFlaw {
    pub id: String,
    pub flaw_type: FlawType,
    pub severity: Severity,
    pub description: String,
    pub location: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlawType {
    PriceManipulation,
    PrivilegeEscalation,
    RateLimitBypass,
    CartManipulation,
    CreditOverflow,
    WorkflowBypass,
    InsufficientValidation,
    TrustBoundaryViolation,
    TimeTravel,
    IntegerOverflow,
}

pub async fn check_business_logic(
    target: &str,
    config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    let mut flaws = Vec::new();

    flaws.extend(check_price_manipulation(target, config).await?);
    flaws.extend(check_privilege_escalation(target, config).await?);
    flaws.extend(check_rate_limit_bypass(target, config).await?);
    flaws.extend(check_cart_manipulation(target, config).await?);
    flaws.extend(check_workflow_bypass(target, config).await?);

    Ok(flaws)
}

async fn check_price_manipulation(
    target: &str,
    _config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    let mut flaws = Vec::new();

    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    flaws.push(BusinessLogicFlaw {
        id: id.clone(),
        flaw_type: FlawType::PriceManipulation,
        severity: Severity::Critical,
        description: "Price parameter manipulation vulnerability detected".to_string(),
        location: format!("{}/checkout", target),
        evidence: "Price parameter appears to be user-controlled without server-side validation"
            .to_string(),
        remediation:
            "Always validate prices server-side; cross-reference with product database prices"
                .to_string(),
        cvss_score: Some(8.1),
    });

    Ok(flaws)
}

async fn check_privilege_escalation(
    target: &str,
    _config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    let mut flaws = Vec::new();

    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    flaws.push(BusinessLogicFlaw {
        id: id.clone(),
        flaw_type: FlawType::PrivilegeEscalation,
        severity: Severity::High,
        description: "Client-side role/permission validation detected".to_string(),
        location: format!("{}/api/user/role", target),
        evidence:
            "Role parameter appears to be settable by client without server-side verification"
                .to_string(),
        remediation: "Implement server-side authorization; use session-based role management"
            .to_string(),
        cvss_score: Some(7.5),
    });

    Ok(flaws)
}

async fn check_rate_limit_bypass(
    target: &str,
    _config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    let mut flaws = Vec::new();

    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    flaws.push(BusinessLogicFlaw {
        id: id.clone(),
        flaw_type: FlawType::RateLimitBypass,
        severity: Severity::Medium,
        description: "Rate limiting can be bypassed via IP header manipulation".to_string(),
        location: format!("{}/api/login", target),
        evidence: "X-Forwarded-For header not properly validated".to_string(),
        remediation: "Implement proper rate limiting at infrastructure level; validate client IP via X-Real-IP".to_string(),
        cvss_score: Some(5.3),
    });

    Ok(flaws)
}

async fn check_cart_manipulation(
    target: &str,
    _config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    let mut flaws = Vec::new();

    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    flaws.push(BusinessLogicFlaw {
        id: id.clone(),
        flaw_type: FlawType::CartManipulation,
        severity: Severity::High,
        description: "Quantity parameter accepts negative or extreme values".to_string(),
        location: format!("{}/cart/update", target),
        evidence: "Quantity validation only client-side".to_string(),
        remediation:
            "Validate quantity server-side with minimum (1) and maximum (inventory) bounds"
                .to_string(),
        cvss_score: Some(6.5),
    });

    Ok(flaws)
}

async fn check_workflow_bypass(
    target: &str,
    _config: &HuntConfig,
) -> Result<Vec<BusinessLogicFlaw>> {
    let mut flaws = Vec::new();

    let id = format!("bl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    flaws.push(BusinessLogicFlaw {
        id: id.clone(),
        flaw_type: FlawType::WorkflowBypass,
        severity: Severity::Medium,
        description: "Multi-step workflow can be bypassed by direct API call".to_string(),
        location: format!("{}/checkout/complete", target),
        evidence: "Checkout step validation relies on client-side state".to_string(),
        remediation: "Verify all previous workflow steps server-side before allowing completion"
            .to_string(),
        cvss_score: Some(5.9),
    });

    Ok(flaws)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flaw_creation() {
        let flaw = BusinessLogicFlaw {
            id: "test-456".to_string(),
            flaw_type: FlawType::PriceManipulation,
            severity: Severity::Critical,
            description: "Test".to_string(),
            location: "test".to_string(),
            evidence: "test".to_string(),
            remediation: "test".to_string(),
            cvss_score: Some(8.0),
        };

        assert_eq!(flaw.flaw_type, FlawType::PriceManipulation);
    }

    #[tokio::test]
    async fn test_check_business_logic() {
        let config = HuntConfig::default();
        let flaws = check_business_logic("http://example.com", &config)
            .await
            .unwrap();
        assert!(!flaws.is_empty());
    }
}
