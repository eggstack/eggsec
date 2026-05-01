use crate::packet::types::*;

impl HttpRequest {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let text = String::from_utf8_lossy(data);
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return None;
        }

        let request_line = lines[0];
        let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            return None;
        }

        let method = parts[0].to_string();
        let uri = parts[1].to_string();
        let version = parts[2].to_string();

        let mut headers = Vec::new();
        let mut body_start = None;

        for (i, line) in lines.iter().skip(1).enumerate() {
            if line.is_empty() {
                body_start = Some(i + 1);
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.push(HttpHeader { name, value });
            }
        }

        let body = body_start.and_then(|start| {
            let body_lines: Vec<&str> = lines.iter().skip(start).copied().collect();
            if body_lines.is_empty() {
                None
            } else {
                Some(body_lines.join("\n").into_bytes())
            }
        });

        Some(Self {
            method,
            uri,
            version,
            headers,
            body,
        })
    }
}

impl HttpResponse {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let text = String::from_utf8_lossy(data);
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return None;
        }

        let status_line = lines[0];
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            return None;
        }

        let version = parts[0].to_string();
        let status_code = parts[1].parse().ok()?;
        let reason_phrase = parts[2].to_string();

        let mut headers = Vec::new();
        let mut body_start = None;

        for (i, line) in lines.iter().skip(1).enumerate() {
            if line.is_empty() {
                body_start = Some(i + 1);
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.push(HttpHeader { name, value });
            }
        }

        let body = body_start.and_then(|start| {
            let body_lines: Vec<&str> = lines.iter().skip(start).copied().collect();
            if body_lines.is_empty() {
                None
            } else {
                Some(body_lines.join("\n").into_bytes())
            }
        });

        Some(Self {
            version,
            status_code,
            reason_phrase,
            headers,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_request() {
        let data = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let request = HttpRequest::parse(data);
        assert!(request.is_some());
        let request = request.unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.uri, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name, "Host");
        assert_eq!(request.headers[0].value, "example.com");
    }

    #[test]
    fn test_parse_http_response() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n";
        let response = HttpResponse::parse(data);
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.version, "HTTP/1.1");
        assert_eq!(response.status_code, 200);
        assert_eq!(response.reason_phrase, "OK");
        assert_eq!(response.headers.len(), 1);
    }

    #[test]
    fn test_parse_empty_data() {
        let data: &[u8] = &[];
        assert!(HttpRequest::parse(data).is_none());
        assert!(HttpResponse::parse(data).is_none());
    }

    #[test]
    fn test_parse_malformed_request() {
        let data = b"NOT A VALID REQUEST";
        assert!(HttpRequest::parse(data).is_none());
    }

    #[test]
    fn test_parse_malformed_response() {
        let data = b"NOT A VALID RESPONSE";
        assert!(HttpResponse::parse(data).is_none());
    }
}
