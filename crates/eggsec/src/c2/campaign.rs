//! Campaign orchestration module (placeholder for future expansion).
//!
//! Supports MITRE ATT&CK profiles and automated campaign runners.
//! Currently delegates to the default campaign definitions in mod.rs.

/// List available campaign profiles.
pub fn available_profiles() -> Vec<(&'static str, &'static str)> {
    vec![
        ("apt29", "APT29 (Cozy Bear) simulation with HTTP/S beacons and LOTL techniques"),
        ("carbanak", "Carbanak/FIN7 simulation with DNS beacons and financial targeting"),
        ("default", "Generic purple team campaign with mixed C2 protocols"),
    ]
}

/// Get a campaign description by profile name.
pub fn profile_description(profile: &str) -> Option<&'static str> {
    available_profiles()
        .into_iter()
        .find(|(name, _)| *name == profile)
        .map(|(_, desc)| desc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_profiles() {
        let profiles = available_profiles();
        assert!(profiles.len() >= 3);
        let names: Vec<_> = profiles.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"apt29"));
        assert!(names.contains(&"carbanak"));
        assert!(names.contains(&"default"));
    }

    #[test]
    fn test_profile_description() {
        assert!(profile_description("apt29").is_some());
        assert!(profile_description("carbanak").is_some());
        assert!(profile_description("unknown").is_none());
    }
}
