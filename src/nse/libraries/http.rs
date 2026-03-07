//! NSE http library wrapper
//!
//! Provides HTTP client functionality compatible with NSE scripts.

use mlua::{Lua, Table};
use std::collections::HashMap;
use std::time::Duration;

fn make_client(timeout_secs: u64) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn parse_options(opts: Option<&Table>) -> (HashMap<String, String>, Duration) {
    let mut headers = HashMap::new();
    let timeout = Duration::from_secs(30);

    if let Some(opts) = opts {
        if let Ok(timeout_val) = opts.get::<u64>("timeout") {
            return (headers, Duration::from_secs(timeout_val));
        }
        if let Ok(headers_table) = opts.get::<Table>("headers") {
            for pair in headers_table.pairs::<String, String>() {
                if let Ok((k, v)) = pair {
                    headers.insert(k, v);
                }
            }
        }
    }

    (headers, timeout)
}

fn build_response(lua: &Lua, resp: reqwest::blocking::Response) -> mlua::Result<Table> {
    let result = lua.create_table()?;

    let status = resp.status().as_u16();
    result.set("status", status as i32)?;

    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let headers_table = lua.create_table()?;
    let mut headers_map = lua.create_table()?;
    for (i, (k, v)) in headers.iter().enumerate() {
        headers_table.set(i + 1, format!("{}: {}", k, v))?;
        headers_map.set(k.clone(), v.clone())?;
    }
    result.set("headers", headers_table)?;
    result.set("header", headers_map)?;

    let version = resp.version();
    result.set("version", format!("{:?}", version))?;

    if status >= 300 && status < 400 {
        if let Some(location) = resp.headers().get("location") {
            if let Ok(loc) = location.to_str() {
                result.set("location", loc.to_string())?;
            }
        }
    }

    if let Ok(body) = resp.text() {
        result.set("body", body)?;
    }

    result.set("https", resp.url().scheme() == "https")?;

    Ok(result)
}

fn error_response(lua: &Lua, err: reqwest::Error) -> mlua::Result<Table> {
    let result = lua.create_table()?;
    result.set("status", 0i32)?;
    result.set("error", err.to_string())?;
    if err.is_timeout() {
        result.set("reason", "timeout")?;
    } else if err.is_connect() {
        result.set("reason", "connection")?;
    } else {
        result.set("reason", "request")?;
    }
    Ok(result)
}

pub fn register_http_library(lua: &Lua) {
    let globals = lua.globals();
    let http = lua.create_table().expect("Failed to create http table");

    http.set(
        "get",
        lua.create_function(
            |lua, (host, port, path, options): (String, u16, String, Option<Table>)| {
                let (_headers, _timeout) = parse_options(options.as_ref());
                let url = if host.starts_with("http") {
                    format!("{}{}", host.trim_end_matches('/'), path)
                } else {
                    format!("http://{}:{}{}", host, port, path)
                };

                let client = make_client(30);

                match client.get(&url).send() {
                    Ok(resp) => build_response(lua, resp),
                    Err(e) => error_response(lua, e),
                }
            },
        ),
    )
    .ok();

    http
        .set(
            "post",
            lua.create_function(
                |lua,
                 (host, port, path, data, options): (
                    String,
                    u16,
                    String,
                    String,
                    Option<Table>,
                )| {
                    let url = if host.starts_with("http") {
                        format!("{}{}", host.trim_end_matches('/'), path)
                    } else {
                        format!("http://{}:{}{}", host, port, path)
                    };

                    let client = make_client(30);

                    match client.post(&url).body(data).send() {
                        Ok(resp) => build_response(lua, resp),
                        Err(e) => error_response(lua, e),
                    }
                },
            ),
        )
        .ok();

    http
        .set(
            "put",
            lua.create_function(
                |lua,
                 (host, port, path, data, options): (
                    String,
                    u16,
                    String,
                    String,
                    Option<Table>,
                )| {
                    let url = if host.starts_with("http") {
                        format!("{}{}", host.trim_end_matches('/'), path)
                    } else {
                        format!("http://{}:{}{}", host, port, path)
                    };

                    let client = make_client(30);

                    match client.put(&url).body(data).send() {
                        Ok(resp) => build_response(lua, resp),
                        Err(e) => error_response(lua, e),
                    }
                },
            ),
        )
        .ok();

    http.set(
        "delete",
        lua.create_function(
            |lua, (host, port, path, options): (String, u16, String, Option<Table>)| {
                let url = if host.starts_with("http") {
                    format!("{}{}", host.trim_end_matches('/'), path)
                } else {
                    format!("http://{}:{}{}", host, port, path)
                };

                let client = make_client(30);

                match client.delete(&url).send() {
                    Ok(resp) => build_response(lua, resp),
                    Err(e) => error_response(lua, e),
                }
            },
        ),
    )
    .ok();

    http.set(
        "head",
        lua.create_function(|lua, (host, port, path): (String, u16, String)| {
            let url = format!("http://{}:{}{}", host, port, path);
            let client = make_client(30);

            match client.head(&url).send() {
                Ok(resp) => {
                    let result = lua.create_table()?;
                    result.set("status", resp.status().as_u16() as i32)?;

                    let headers: Vec<(String, String)> = resp
                        .headers()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                        .collect();

                    let headers_table = lua.create_table()?;
                    for (i, (k, v)) in headers.iter().enumerate() {
                        headers_table.set(i + 1, format!("{}: {}", k, v))?;
                    }
                    result.set("headers", headers_table)?;

                    Ok(result)
                }
                Err(e) => error_response(lua, e),
            }
        }),
    )
    .ok();

    http.set(
        "options",
        lua.create_function(|lua, (host, port, path): (String, u16, String)| {
            let url = format!("http://{}:{}{}", host, port, path);
            let client = make_client(30);

            match client.request(reqwest::Method::OPTIONS, &url).send() {
                Ok(resp) => build_response(lua, resp),
                Err(e) => error_response(lua, e),
            }
        }),
    )
    .ok();

    http
        .set(
            "request",
            lua.create_function(
                |lua,
                 (method, host, port, path, options): (
                    String,
                    String,
                    u16,
                    String,
                    Option<Table>,
                )| {
                    let url = if host.starts_with("http") {
                        format!("{}{}", host.trim_end_matches('/'), path)
                    } else {
                        format!("http://{}:{}{}", host, port, path)
                    };

                    let client = make_client(30);

                    let mut req =
                        client.request(method.parse().unwrap_or(reqwest::Method::GET), &url);

                    if let Some(opts) = options {
                        if let Ok(body) = opts.get::<String>("body") {
                            req = req.body(body);
                        }
                        if let Ok(headers) = opts.get::<Table>("headers") {
                            for pair in headers.pairs::<String, String>() {
                                if let Ok((k, v)) = pair {
                                    req = req.header(&k, &v);
                                }
                            }
                        }
                        if let Ok(auth) = opts.get::<String>("authorization") {
                            req = req.header("Authorization", &auth);
                        }
                        if let Ok(ua) = opts.get::<String>("useragent") {
                            req = req.header("User-Agent", &ua);
                        }
                    }

                    match req.send() {
                        Ok(resp) => build_response(lua, resp),
                        Err(e) => error_response(lua, e),
                    }
                },
            ),
        )
        .ok();

    http.set(
        "pipeline",
        lua.create_function(|lua, (requests, options): (Vec<Table>, Option<Table>)| {
            let results = lua.create_table()?;
            let client = make_client(60);

            for (i, req) in requests.iter().enumerate() {
                let method: String = req.get("method").unwrap_or_else(|_| "GET".to_string());
                let host: String = req.get("host").unwrap_or_else(|_| "".to_string());
                let port: u16 = req.get::<u16>("port").unwrap_or(80);
                let path: String = req.get("path").unwrap_or_else(|_| "/".to_string());
                let body: String = req.get("body").unwrap_or_else(|_| "".to_string());

                let url = if host.starts_with("http") {
                    format!("{}{}", host.trim_end_matches('/'), path)
                } else {
                    format!("http://{}:{}{}", host, port, path)
                };

                let result = match method.to_uppercase().as_str() {
                    "GET" => client.get(&url).send(),
                    "POST" => client.post(&url).body(body).send(),
                    "PUT" => client.put(&url).body(body).send(),
                    "DELETE" => client.delete(&url).send(),
                    "HEAD" => client.head(&url).send(),
                    "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url).send(),
                    _ => client.get(&url).send(),
                };

                let result_table = match result {
                    Ok(resp) => build_response(lua, resp),
                    Err(e) => error_response(lua, e),
                }?;

                results.set(i + 1, result_table)?;
            }

            Ok(results)
        }),
    )
    .ok();

    http.set("version", lua.create_function(|_lua, _: ()| Ok("1.0.0")))
        .ok();

    http.set(
        "validate",
        lua.create_function(|lua, response: Table| {
            let status: i32 = response.get("status").unwrap_or(0);
            let body: String = response.get("body").unwrap_or_default();

            let result = lua.create_table()?;
            result.set("valid", status >= 200 && status < 400)?;
            result.set("status", status)?;
            result.set("has_body", !body.is_empty())?;

            if let Ok(headers) = response.get::<Table>("headers") {
                let content_type: Option<String> = headers.get("content-type").ok();
                result.set("content_type", content_type.unwrap_or_default())?;
            }

            Ok(result)
        }),
    )
    .ok();

    globals.set("http", http).ok();
}
