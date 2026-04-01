use crate::config::Scope;
use crate::utils::target::extract_target_from_url;

pub fn check_scope(scope: &Scope, target: &str) -> anyhow::Result<()> {
    if !scope.is_target_allowed(target)? {
        anyhow::bail!("Target {} is not in allowed scope", target);
    }
    Ok(())
}

pub fn check_scope_from_url(scope: &Scope, url: &str) -> anyhow::Result<()> {
    let target = extract_target_from_url(url)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse URL: {}", url))?;
    check_scope(scope, &target)
}
