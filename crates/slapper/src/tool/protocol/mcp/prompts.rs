use serde::{Deserialize, Serialize};

use super::profile::McpProfile;

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

pub fn get_ops_agent_prompts() -> Vec<McpPrompt> {
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

pub fn get_coding_agent_prompts() -> Vec<McpPrompt> {
    vec![
        McpPrompt {
            name: "live-validation-before-merge".to_string(),
            description: "Guide bounded validation before finalizing web/API changes".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "change_description".to_string(),
                    description: "Description of the change".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target URL or host to validate against".to_string(),
                    required: true,
                },
            ],
            template: "Guide bounded validation for the following change before merge:\n\nChange: {{change_description}}\nTarget: {{target}}"
                .to_string(),
        },
        McpPrompt {
            name: "auth-change-validation".to_string(),
            description: "Guide validation of changed auth/session/permission logic".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "change_description".to_string(),
                    description: "Description of the auth change".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target URL or host to validate against".to_string(),
                    required: true,
                },
            ],
            template: "Guide validation of the following auth/session/permission change:\n\nChange: {{change_description}}\nTarget: {{target}}"
                .to_string(),
        },
        McpPrompt {
            name: "api-change-validation".to_string(),
            description: "Guide validation of changed API schema/routes/input handling".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "change_description".to_string(),
                    description: "Description of the API change".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target URL or host to validate against".to_string(),
                    required: true,
                },
            ],
            template: "Guide validation of the following API schema/route/input change:\n\nChange: {{change_description}}\nTarget: {{target}}"
                .to_string(),
        },
        McpPrompt {
            name: "file-surface-validation".to_string(),
            description: "Guide validation of upload/download/import/export features".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "change_description".to_string(),
                    description: "Description of the file surface change".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target URL or host to validate against".to_string(),
                    required: true,
                },
            ],
            template: "Guide validation of the following upload/download/import/export feature:\n\nChange: {{change_description}}\nTarget: {{target}}"
                .to_string(),
        },
        McpPrompt {
            name: "security-regression-retest".to_string(),
            description: "Guide use of retest_finding after a patch".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "finding_id".to_string(),
                    description: "Finding ID to retest".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target URL or host to retest against".to_string(),
                    required: true,
                },
            ],
            template: "Guide retest of finding {{finding_id}} against target {{target}} after patch."
                .to_string(),
        },
        McpPrompt {
            name: "interpret-dynamic-finding-for-code-fix".to_string(),
            description: "Help interpret Slapper evidence and map to patch planning".to_string(),
            arguments: vec![
                PromptArgument {
                    name: "finding".to_string(),
                    description: "Dynamic finding evidence to interpret".to_string(),
                    required: true,
                },
                PromptArgument {
                    name: "target".to_string(),
                    description: "Target URL or host where finding was observed".to_string(),
                    required: true,
                },
            ],
            template: "Interpret the following Slapper finding and map it to a code fix plan:\n\nFinding: {{finding}}\nTarget: {{target}}"
                .to_string(),
        },
    ]
}

pub fn get_builtin_prompts_for_profile(profile: &McpProfile) -> Vec<McpPrompt> {
    match profile {
        McpProfile::OpsAgent => get_ops_agent_prompts(),
        McpProfile::CodingAgent => get_coding_agent_prompts(),
    }
}

pub fn get_builtin_prompts() -> Vec<McpPrompt> {
    get_ops_agent_prompts()
}
