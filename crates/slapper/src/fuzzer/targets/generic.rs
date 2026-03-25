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
