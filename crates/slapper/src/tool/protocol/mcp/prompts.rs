use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

pub fn get_builtin_prompts() -> Vec<McpPrompt> {
    vec![
        McpPrompt {
            name: "vulnerability-analysis".to_string(),
            description: "Analyze a vulnerability finding".to_string(),
            arguments: vec![PromptArgument {
                name: "finding".to_string(),
                description: "Vulnerability description".to_string(),
                required: true,
            }],
            template: "Analyze the following vulnerability: {{finding}}".to_string(),
        },
        McpPrompt {
            name: "attack-chain".to_string(),
            description: "Identify potential attack chains".to_string(),
            arguments: vec![PromptArgument {
                name: "findings".to_string(),
                description: "List of findings".to_string(),
                required: true,
            }],
            template: "Given these findings: {{findings}}\n\nIdentify potential attack chains."
                .to_string(),
        },
        McpPrompt {
            name: "remediation".to_string(),
            description: "Generate remediation guidance".to_string(),
            arguments: vec![PromptArgument {
                name: "vulnerability".to_string(),
                description: "Vulnerability type".to_string(),
                required: true,
            }],
            template: "Provide detailed remediation guidance for: {{vulnerability}}".to_string(),
        },
        McpPrompt {
            name: "scope-check".to_string(),
            description: "Verify if a target is in scope".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target to check".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "scope".to_string(),
                    description: "Scope rules".to_string(),
                    required: true,
                },
            ],
            template: "Is {{target}} within scope? Scope rules: {{scope}}".to_string(),
        },
        McpPrompt {
            name: "report-summary".to_string(),
            description: "Generate executive summary".to_string(),
            arguments: vec![PromptArgument {
                name: "findings".to_string(),
                description: "All findings".to_string(),
                required: true,
            }],
            template: "Generate an executive summary for these findings: {{findings}}".to_string(),
        },
        McpPrompt {
            name: "payload-suggestion".to_string(),
            description: "Suggest fuzzing payloads".to_string(),
            arguments: vec![PromptArgument {
                name: "vuln_type".to_string(),
                description: "Vulnerability type".to_string(),
                required: true,
            }],
            template: "Suggest payloads for {{vuln_type}}".to_string(),
        },
        McpPrompt {
            name: "waf-bypass".to_string(),
            description: "Suggest WAF bypass techniques".to_string(),
            arguments: vec![PromptArgument {
                name: "waf".to_string(),
                description: "WAF product".to_string(),
                required: true,
            }],
            template: "Suggest WAF bypass techniques for {{waf}}".to_string(),
        },
    ]
}
