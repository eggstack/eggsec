use super::TargetPayload;

pub fn get_payloads() -> Vec<TargetPayload> {
    let mut payloads = Vec::new();

    payloads.extend(get_debug_payloads());
    payloads.extend(get_info_disclosure_payloads());
    payloads.extend(get_backup_payloads());
    payloads.extend(get_svn_git_payloads());

    payloads
}

fn get_debug_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/debug".to_string(),
            description: "Debug endpoint".to_string(),
            category: "debug".to_string(),
        },
        TargetPayload {
            payload: "/test".to_string(),
            description: "Test endpoint".to_string(),
            category: "debug".to_string(),
        },
        TargetPayload {
            payload: "/trace".to_string(),
            description: "Trace endpoint".to_string(),
            category: "debug".to_string(),
        },
        TargetPayload {
            payload: "/api/debug".to_string(),
            description: "API debug endpoint".to_string(),
            category: "debug".to_string(),
        },
        TargetPayload {
            payload: "/_debug".to_string(),
            description: "Underscore debug".to_string(),
            category: "debug".to_string(),
        },
        TargetPayload {
            payload: "/console".to_string(),
            description: "Console endpoint".to_string(),
            category: "debug".to_string(),
        },
    ]
}

fn get_info_disclosure_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/phpinfo.php".to_string(),
            description: "PHP info disclosure".to_string(),
            category: "info-disclosure".to_string(),
        },
        TargetPayload {
            payload: "/info.php".to_string(),
            description: "Info PHP file".to_string(),
            category: "info-disclosure".to_string(),
        },
        TargetPayload {
            payload: "/server-status".to_string(),
            description: "Server status".to_string(),
            category: "info-disclosure".to_string(),
        },
        TargetPayload {
            payload: "/elmah.axd".to_string(),
            description: "ELMAH error log".to_string(),
            category: "info-disclosure".to_string(),
        },
        TargetPayload {
            payload: "/trace.axd".to_string(),
            description: "ASP.NET trace".to_string(),
            category: "info-disclosure".to_string(),
        },
        TargetPayload {
            payload: "/.svn/entries".to_string(),
            description: "SVN entries file".to_string(),
            category: "info-disclosure".to_string(),
        },
        TargetPayload {
            payload: "/.git/config".to_string(),
            description: "Git config file".to_string(),
            category: "info-disclosure".to_string(),
        },
    ]
}

fn get_backup_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "index.php.bak".to_string(),
            description: "PHP backup file".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "index.php~".to_string(),
            description: "Vim swap file".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "index.php.swp".to_string(),
            description: "Vim swap file".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "index.php.old".to_string(),
            description: "Old PHP file".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "index.php.bak".to_string(),
            description: "Bak extension".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "backup.sql".to_string(),
            description: "SQL backup file".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "database.sql".to_string(),
            description: "Database dump".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "dump.sql".to_string(),
            description: "SQL dump".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "backup.zip".to_string(),
            description: "Backup archive".to_string(),
            category: "backup".to_string(),
        },
        TargetPayload {
            payload: "backup.tar.gz".to_string(),
            description: "Tarball backup".to_string(),
            category: "backup".to_string(),
        },
    ]
}

fn get_svn_git_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/.git/HEAD".to_string(),
            description: "Git HEAD file".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.git/config".to_string(),
            description: "Git config".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.git/objects/".to_string(),
            description: "Git objects directory".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.gitignore".to_string(),
            description: "Git ignore file".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.svn/entries".to_string(),
            description: "SVN entries".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.svn/wc.db".to_string(),
            description: "SVN working copy database".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.hg/store/data/".to_string(),
            description: "Mercurial data".to_string(),
            category: "vcs".to_string(),
        },
        TargetPayload {
            payload: "/.bzr/checkout/".to_string(),
            description: "Bazaar checkout".to_string(),
            category: "vcs".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_payloads_non_empty() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 25,
            "generic should have at least 25 payloads, got {}",
            payloads.len()
        );
    }

    #[test]
    fn test_generic_categories() {
        let payloads = get_payloads();
        let categories: Vec<&str> = payloads.iter().map(|p| p.category.as_str()).collect();
        assert!(categories.contains(&"debug"));
        assert!(categories.contains(&"info-disclosure"));
        assert!(categories.contains(&"backup"));
        assert!(categories.contains(&"vcs"));
    }

    #[test]
    fn test_generic_debug_count() {
        let payloads = get_debug_payloads();
        assert_eq!(payloads.len(), 6);
    }

    #[test]
    fn test_generic_info_disclosure_count() {
        let payloads = get_info_disclosure_payloads();
        assert_eq!(payloads.len(), 7);
    }

    #[test]
    fn test_generic_backup_count() {
        let payloads = get_backup_payloads();
        assert_eq!(payloads.len(), 10);
    }

    #[test]
    fn test_generic_vcs_count() {
        let payloads = get_svn_git_payloads();
        assert_eq!(payloads.len(), 8);
    }

    #[test]
    fn test_generic_all_payloads_have_required_fields() {
        for p in get_payloads() {
            assert!(!p.payload.is_empty());
            assert!(!p.description.is_empty());
            assert!(!p.category.is_empty());
        }
    }

    #[test]
    fn test_generic_vcs_includes_git_and_svn() {
        let payloads = get_svn_git_payloads();
        let payload_strs: Vec<&str> = payloads.iter().map(|p| p.payload.as_str()).collect();
        assert!(payload_strs.iter().any(|p| p.contains(".git")));
        assert!(payload_strs.iter().any(|p| p.contains(".svn")));
    }
}
