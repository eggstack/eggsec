use crate::config::Scope;
use crate::error::{Result, SlapperError};
use crate::utils::target::extract_target_from_url;

pub fn check_scope(scope: &Scope, target: &str) -> Result<()> {
    if !scope.is_target_allowed(target)? {
        return Err(SlapperError::ScopeViolation(format!(
            "Target {} is not in allowed scope",
            target
        )));
    }
    Ok(())
}

pub fn check_scope_from_url(scope: &Scope, url: &str) -> Result<()> {
    let target = extract_target_from_url(url)
        .ok_or_else(|| SlapperError::Parse(format!("Failed to parse URL: {}", url)))?;
    check_scope(scope, &target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScopeRule;

    #[test]
    fn test_check_scope_allowed() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));

        assert!(check_scope(&scope, "example.com").is_ok());
    }

    #[test]
    fn test_check_scope_denied() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));

        let result = check_scope(&scope, "other.com");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in allowed scope"));
    }

    #[test]
    fn test_check_scope_empty_scope_allows_all() {
        let scope = Scope::new();
        assert!(check_scope(&scope, "anything.com").is_ok());
    }

    #[test]
    fn test_check_scope_require_explicit_denies_when_empty() {
        let mut scope = Scope::new();
        scope.require_explicit_scope = true;

        let result = check_scope(&scope, "example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_scope_from_url_valid() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));

        assert!(check_scope_from_url(&scope, "https://example.com/path").is_ok());
    }

    #[test]
    fn test_check_scope_from_url_denied() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));

        let result = check_scope_from_url(&scope, "http://evil.com/page");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_scope_from_url_empty_url() {
        let scope = Scope::new();
        let result = check_scope_from_url(&scope, "");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_scope_from_url_no_host() {
        let scope = Scope::new();
        let result = check_scope_from_url(&scope, "file:///etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_scope_wildcard() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("*.example.com".to_string()));

        assert!(check_scope(&scope, "sub.example.com").is_ok());
        assert!(check_scope(&scope, "example.com").is_ok());
        let result = check_scope(&scope, "other.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_scope_excluded() {
        let mut scope = Scope::new();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));
        scope
            .excluded_targets
            .push(ScopeRule::new("admin.example.com".to_string()));

        assert!(check_scope(&scope, "example.com").is_ok());
        let result = check_scope(&scope, "admin.example.com");
        assert!(result.is_err());
    }
}
