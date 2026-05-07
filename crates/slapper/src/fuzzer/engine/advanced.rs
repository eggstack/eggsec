use crate::error::{Result, SlapperError};
use std::time::Duration;

use super::super::advanced::{
    AdvancedFuzzer, GraphQLFuzzer, GrpcFuzzer, IdorFuzzer, JwtFuzzer, OAuthFuzzer, SstiFuzzer,
    WebSocketFuzzer,
};
use super::super::payloads::PayloadType;
use super::types::FuzzResult;

use super::core::FuzzEngine;

impl FuzzEngine {
    pub(crate) async fn run_advanced_fuzzer(&self, fuzzer_type: &str) -> Result<Vec<FuzzResult>> {
        let insecure = self.args.common.insecure;
        if insecure {
            tracing::warn!(
                "TLS certificate verification disabled. This is insecure and should only \
                 be used in isolated testing environments."
            );
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.args.timeout))
            .danger_accept_invalid_certs(insecure)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| {
                crate::error::SlapperError::from(e).with_timeout(self.args.timeout * 1000)
            })?;

        match fuzzer_type {
            "graphql" => {
                let mut fuzzer = GraphQLFuzzer::new(self.args.url.clone())
                    .with_introspection(self.args.graphql_introspection)
                    .with_depth_bypass(self.args.graphql_depth_bypass)
                    .with_alias_overload(self.args.graphql_alias_overload);
                Ok(fuzzer.fuzz(&client).await)
            }
            "jwt" => {
                let mut fuzzer = JwtFuzzer::new().with_target_url(self.args.url.clone());

                if let Some(ref token) = self.args.jwt_token {
                    fuzzer = fuzzer.with_original_token(token.clone());
                }

                Ok(fuzzer.fuzz(&client).await)
            }
            "oauth" => {
                let client_id = self
                    .args
                    .oauth_client_id
                    .clone()
                    .unwrap_or_else(|| "test-client-id".to_string());
                let client_secret = self
                    .args
                    .oauth_client_secret
                    .clone()
                    .unwrap_or_else(|| "test-client-secret".to_string());
                let redirect_uri = "http://localhost/callback".to_string();

                let mut fuzzer = OAuthFuzzer::new(client_id, redirect_uri)
                    .with_client_secret(client_secret)
                    .with_redirect_test(self.args.oauth_redirect)
                    .with_scope_test(self.args.oauth_scope)
                    .with_state_test(self.args.oauth_state)
                    .with_grant_test(self.args.oauth_grant);

                if let Some(ref issuer) = self.args.oauth_issuer {
                    fuzzer = fuzzer.with_issuer(issuer.clone());
                }

                Ok(fuzzer.fuzz(&client).await)
            }
            "idor" => {
                let mut fuzzer = IdorFuzzer::new(self.args.url.clone());

                if let Some(ref base_id) = self.args.idor_base_id {
                    fuzzer = fuzzer.with_base_user_id(base_id.clone());
                }

                if let Some(ref user_ids_str) = self.args.idor_user_ids {
                    let ids: Vec<String> = user_ids_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    fuzzer = fuzzer.with_user_ids(ids);
                }

                Ok(fuzzer.fuzz(&client).await)
            }
            "ssti" => {
                let mut fuzzer = SstiFuzzer::new().with_target_url(self.args.url.clone());

                if let Some(ref param) = self.args.ssti_param {
                    fuzzer = fuzzer.with_param_name(param.clone());
                }

                Ok(fuzzer.fuzz(&client).await)
            }
            "websocket" => {
                let mut fuzzer = WebSocketFuzzer::new(self.args.url.clone());
                fuzzer
                    .fuzz(&client)
                    .await
                    .map_err(crate::error::SlapperError::from)
            }
            "grpc" => {
                let mut fuzzer = GrpcFuzzer::new(self.args.url.clone());
                fuzzer
                    .fuzz(&client)
                    .await
                    .map_err(crate::error::SlapperError::from)
            }
            _ => Ok(Vec::new()),
        }
    }

    pub(crate) fn parse_payload_types(&self) -> Result<Vec<PayloadType>> {
        if self.args.payload_type == "all" {
            return Ok(PayloadType::all_variants().to_vec());
        }

        let types: Vec<PayloadType> = self
            .args
            .payload_type
            .split(',')
            .filter_map(|s| match s.trim().to_lowercase().as_str() {
                "sqli" | "sql" => Some(PayloadType::Sqli),
                "xss" => Some(PayloadType::Xss),
                "traversal" | "lfi" | "path" => Some(PayloadType::Traversal),
                "ssrf" => Some(PayloadType::Ssrf),
                "redirect" | "open-redirect" => Some(PayloadType::Redirect),
                "redos" | "regex" => Some(PayloadType::Redos),
                "headers" | "header" => Some(PayloadType::Headers),
                "compression" | "gzip" | "zip-bomb" => Some(PayloadType::Compression),
                "graphql" | "gql" => Some(PayloadType::GraphQL),
                "oauth" | "oidc" => Some(PayloadType::OAuth),
                "jwt" => Some(PayloadType::Jwt),
                "idor" | "auth" => Some(PayloadType::Idor),
                "ssti" | "template" => Some(PayloadType::Ssti),
                "grpc" | "protobuf" => Some(PayloadType::Grpc),
                "xxe" | "xml" => Some(PayloadType::Xxe),
                "ldap" => Some(PayloadType::Ldap),
                "cmd" | "command" | "rce" => Some(PayloadType::Cmd),
                "deser" | "deserialization" => Some(PayloadType::Deser),
                "host" | "host-header" => Some(PayloadType::Host),
                "cache" | "cache-poisoning" => Some(PayloadType::Cache),
                "csv" | "formula" => Some(PayloadType::Csv),
                "soap" => Some(PayloadType::Soap),
                _ => None,
            })
            .collect();

        if types.is_empty() {
            return Err(SlapperError::Payload(
                "No valid payload types specified".to_string(),
            ));
        }

        Ok(types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{CommonHttpArgs, FuzzArgs};
    use crate::error::SlapperError;

    fn make_engine_with_payload_type(payload_type: &str) -> FuzzEngine {
        let args = FuzzArgs {
            url: "http://example.com".to_string(),
            payload_type: payload_type.to_string(),
            common: CommonHttpArgs::default(),
            method: "GET".to_string(),
            param: None,
            concurrency: 10,
            timeout: 5,
            verbose: false,
            quiet: false,
            json: false,
            output: None,
            mutate: false,
            mutation_count: 5,
            grammar_fuzz: false,
            grammar_type: None,
            session: false,
            diffing: false,
            capture_baseline: false,
            mode: crate::cli::FuzzMode::Sequential,
            target: None,
            graphql_introspection: false,
            graphql_depth_bypass: false,
            graphql_alias_overload: false,
            jwt_token: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_redirect: false,
            oauth_scope: false,
            oauth_state: false,
            oauth_grant: false,
            oauth_issuer: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            adaptive_rate: false,
            enhanced_redos: false,
            waf_fingerprint: false,
            chaining: false,
            chain_file: None,
            format: None,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
        };
        FuzzEngine::new(args).unwrap()
    }

    #[test]
    fn test_parse_payload_types_all() {
        let engine = make_engine_with_payload_type("all");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types.len(), PayloadType::all_variants().len());
        assert!(types.contains(&PayloadType::Sqli));
        assert!(types.contains(&PayloadType::Xss));
        assert!(types.contains(&PayloadType::Ssrf));
        assert!(types.contains(&PayloadType::Jwt));
        assert!(types.contains(&PayloadType::Oast));
        assert!(types.contains(&PayloadType::Websocket));
    }

    #[test]
    fn test_parse_payload_types_single() {
        let engine = make_engine_with_payload_type("sqli");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0], PayloadType::Sqli);
    }

    #[test]
    fn test_parse_payload_types_multiple() {
        let engine = make_engine_with_payload_type("sqli,xss,ssrf");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types.len(), 3);
        assert_eq!(types[0], PayloadType::Sqli);
        assert_eq!(types[1], PayloadType::Xss);
        assert_eq!(types[2], PayloadType::Ssrf);
    }

    #[test]
    fn test_parse_payload_types_with_aliases() {
        let engine = make_engine_with_payload_type("sql,lfi,rce");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types[0], PayloadType::Sqli);
        assert_eq!(types[1], PayloadType::Traversal);
        assert_eq!(types[2], PayloadType::Cmd);
    }

    #[test]
    fn test_parse_payload_types_with_whitespace() {
        let engine = make_engine_with_payload_type(" sqli , xss ");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn test_parse_payload_types_case_insensitive() {
        let engine = make_engine_with_payload_type("SQLI,XSS");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0], PayloadType::Sqli);
        assert_eq!(types[1], PayloadType::Xss);
    }

    #[test]
    fn test_parse_payload_types_invalid_returns_error() {
        let engine = make_engine_with_payload_type("invalid_type");
        let result = engine.parse_payload_types();
        assert!(result.is_err());
        match result.unwrap_err() {
            SlapperError::Payload(msg) => {
                assert!(msg.contains("No valid payload types"));
            }
            _ => panic!("Expected Payload error"),
        }
    }

    #[test]
    fn test_parse_payload_types_mixed_valid_invalid() {
        let engine = make_engine_with_payload_type("sqli,invalid,xss");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0], PayloadType::Sqli);
        assert_eq!(types[1], PayloadType::Xss);
    }

    #[test]
    fn test_parse_payload_types_graphql_alias() {
        let engine = make_engine_with_payload_type("gql");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types[0], PayloadType::GraphQL);
    }

    #[test]
    fn test_parse_payload_types_redirect_alias() {
        let engine = make_engine_with_payload_type("open-redirect");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types[0], PayloadType::Redirect);
    }

    #[test]
    fn test_parse_payload_types_redos_alias() {
        let engine = make_engine_with_payload_type("regex");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types[0], PayloadType::Redos);
    }

    #[test]
    fn test_parse_payload_types_compression_aliases() {
        let engine = make_engine_with_payload_type("gzip");
        let types = engine.parse_payload_types().unwrap();
        assert_eq!(types[0], PayloadType::Compression);
    }

    #[test]
    fn test_parse_payload_types_empty_string_returns_error() {
        let engine = make_engine_with_payload_type("");
        let result = engine.parse_payload_types();
        assert!(result.is_err());
    }
}
