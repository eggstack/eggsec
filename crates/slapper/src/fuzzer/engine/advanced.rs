use anyhow::Result;
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
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.args.timeout))
            .danger_accept_invalid_certs(insecure)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

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
                fuzzer.fuzz(&client).await
            }
            "grpc" => {
                let mut fuzzer = GrpcFuzzer::new(self.args.url.clone());
                fuzzer.fuzz(&client).await
            }
            _ => Ok(Vec::new()),
        }
    }

    pub(crate) fn parse_payload_types(&self) -> Result<Vec<PayloadType>> {
        if self.args.payload_type == "all" {
            return Ok(vec![
                PayloadType::Sqli,
                PayloadType::Xss,
                PayloadType::Traversal,
                PayloadType::Ssrf,
                PayloadType::Redirect,
                PayloadType::Redos,
                PayloadType::Headers,
                PayloadType::Compression,
                PayloadType::GraphQL,
                PayloadType::OAuth,
                PayloadType::Jwt,
                PayloadType::Idor,
                PayloadType::Ssti,
                PayloadType::Grpc,
                PayloadType::Xxe,
                PayloadType::Ldap,
                PayloadType::Cmd,
                PayloadType::Deser,
                PayloadType::Host,
                PayloadType::Cache,
                PayloadType::Csv,
                PayloadType::Soap,
            ]);
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
            anyhow::bail!("No valid payload types specified");
        }

        Ok(types)
    }
}
