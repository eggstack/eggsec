#![allow(dead_code)]

use crate::config::Scope;
use crate::utils::target::extract_target_from_url;

pub fn check_scope(scope: &Scope, target: &str) -> anyhow::Result<()> {
    if !scope.is_target_allowed(target)? {
        anyhow::bail!("Target {} is not in allowed scope", target);
    }
    Ok(())
}

pub fn check_scope_from_url(scope: &Scope, url: &str) -> anyhow::Result<()> {
    if let Some(target) = extract_target_from_url(url) {
        check_scope(scope, &target)?;
    }
    Ok(())
}
