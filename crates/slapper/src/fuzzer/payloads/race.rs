use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Race,
        "time-of-check", [
            ("send message - race condition", "TOCTOU - send message race", Severity::High),
            ("transfer funds - race", "TOCTOU - fund transfer race", Severity::Critical),
            ("increment counter - race", "TOCTOU - counter increment race", Severity::High),
            ("check then set - race", "TOCTOU - check-set race", Severity::High),
            ("auth bypass - race", "TOCTOU - authentication bypass", Severity::Critical),
            ("session race - race", "TOCTOU - session race", Severity::Critical),
            ("privilege escalation - race", "TOCTOU - privilege escalation", Severity::Critical),
            ("stock decrement - race", "TOCTOU - stock decrement race", Severity::High),
            ("coupon reuse - race", "TOCTOU - coupon reuse race", Severity::High),
            ("password change - race", "TOCTOU - password change race", Severity::High),
        ];
        "http-concurrent", [
            ("send 100 concurrent requests", "100 concurrent requests", Severity::High),
            ("send 50 parallel requests", "50 parallel requests", Severity::Medium),
            ("send 200 concurrent POST", "200 concurrent POST", Severity:: High),
            ("send 1000 overlapping requests", "1000 overlapping requests", Severity::Critical),
            ("rapid-fire same endpoint", "Rapid-fire same endpoint", Severity::High),
            ("concurrent different methods", "Concurrent different methods", Severity:: Medium),
        ];
        "race-primitives", [
            ("{{timestamp}}", "Timestamp substitution", Severity::High),
            ("{{uuid}}", "UUID substitution", Severity::High),
            ("{{random}}", "Random value substitution", Severity::Medium),
            ("{{nonce}}", "Nonce substitution", Severity::High),
            ("{{counter}}", "Counter substitution", Severity::High),
            ("{{epoch}}", "Epoch time substitution", Severity::Medium),
        ];
        "header-races", [
            ("X-Forwarded-For: {{random}}", "X-Forwarded-For race", Severity::Medium),
            ("X-Original-IP: {{random}}", "X-Original-IP race", Severity::Medium),
            ("CF-Connecting-IP: {{random}}", "CF-Connecting-IP race", Severity:: Medium),
            ("True-Client-IP: {{random}}", "True-Client-IP race", Severity::Medium),
            ("X-Real-IP: {{random}}", "X-Real-IP race", Severity::Medium),
        ];
        "auth-races", [
            ("auth bypass via race", "Authentication race condition", Severity::Critical),
            ("session fixation race", "Session fixation race", Severity::Critical),
            ("CSRF token race", "CSRF token race", Severity::High),
            ("OAuth state race", "OAuth state parameter race", Severity::High),
            ("JWT race condition", "JWT race condition", Severity::Critical),
            ("reset token reuse race", "Reset token reuse race", Severity::Critical),
        ];
    );

    for p in &mut payloads {
        if !p.tags.contains(&"race-condition".to_string()) {
            p.tags.push("race-condition".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "Race condition payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_race_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Race);
        }
    }

    #[test]
    fn contains_concurrent_patterns() {
        let payloads = get_payloads();
        let has_concurrent = payloads.iter().any(|p| {
            p.payload.contains("concurrent") || p.payload.contains("parallel")
        });
        assert!(has_concurrent, "Must contain concurrent payload patterns");
    }

    #[test]
    fn contains_race_indicators() {
        let payloads = get_payloads();
        let has_race = payloads.iter().any(|p| {
            p.payload.contains("race") || p.payload.contains("TOCTOU")
        });
        assert!(has_race, "Must contain race condition indicators");
    }

    #[test]
    fn contains_auth_races() {
        let payloads = get_payloads();
        let has_auth = payloads.iter().any(|p| {
            p.payload.contains("auth") || p.payload.contains("session") || p.payload.contains("JWT")
        });
        assert!(has_auth, "Must contain authentication race payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 20,
            "Must have substantial race condition coverage, got {}",
            payloads.len()
        );
    }
}
