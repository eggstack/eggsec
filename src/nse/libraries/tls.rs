//! NSE tls library wrapper
//!
//! Provides TLS/SSL protocol utilities and parsing.
//! Based on Nmap's tls library: https://nmap.org/nsedoc/lib/tls.html

use mlua::Lua;
use std::net::TcpStream;

pub fn register_tls_library(lua: &Lua) {
    let globals = lua.globals();

    let tls = lua.create_table().expect("Failed to create tls table");

    tls.set(
        "get_clients",
        lua.create_function(|_lua, _: ()| Ok(vec!["TLS 1.3", "TLS 1.2", "TLS 1.1", "TLS 1.0"]))
            .ok(),
    );

    tls.set(
        "get_servers",
        lua.create_function(|_lua, _: ()| {
            Ok(vec!["TLS 1.3", "TLS 1.2", "TLS 1.1", "TLS 1.0", "SSL 3.0"])
        })
        .ok(),
    );

    tls.set(
        "probe_socket",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let result = lua.create_table().expect("Failed to create result table");

            let connector = match native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = result.set("error", format!("TLS connector error: {}", e));
                    return Ok(result);
                }
            };

            let stream = match TcpStream::connect(format!("{}:{}", host, port)) {
                Ok(s) => s,
                Err(e) => {
                    let _ = result.set("error", format!("Connection error: {}", e));
                    return Ok(result);
                }
            };

            let tls_stream = match connector.connect(&host, stream) {
                Ok(t) => t,
                Err(e) => {
                    let _ = result.set("error", format!("TLS handshake error: {}", e));
                    return Ok(result);
                }
            };

            let _ = result.set("valid", true);
            let _ = result.set("host", host);
            let _ = result.set("port", port);

            Ok(result)
        })
        .ok(),
    );

    tls.set(
        "parse_certificate",
        lua.create_function(|lua, cert_pem: String| {
            let sslcert = lua.globals().get::<mlua::Table>("sslcert");
            if let Ok(sslcert) = sslcert {
                if let Ok(parse_fn) = sslcert.get::<mlua::Function>("parse_certificate") {
                    return parse_fn.call(cert_pem);
                }
            }
            let result = lua.create_table().expect("Failed to create result table");
            let _ = result.set("error", "parse_certificate function not found");
            Ok(result)
        })
        .ok(),
    );

    globals.set("tls", tls).ok();
}
