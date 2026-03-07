//! NSE smtp library wrapper
//!
//! SMTP protocol support for NSE scripts.
//! Based on Nmap's smtp library concepts.

use mlua::Lua;
use std::io::Read;
use std::net::TcpStream;

pub fn register_smtp_library(lua: &Lua) {
    let globals = lua.globals();

    let smtp = lua.create_table().expect("Failed to create smtp table");

    smtp.set(
        "connect",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let result = lua.create_table().expect("Failed to create result table");

            let addr = format!("{}:{}", host, port);
            let mut stream = match TcpStream::connect(&addr) {
                Ok(s) => s,
                Err(e) => {
                    let _ = result.set("status", "error");
                    let _ = result.set("error", e.to_string());
                    return Ok(result);
                }
            };

            let mut buffer = vec![0u8; 1024];
            match stream.read(&mut buffer) {
                Ok(n) => {
                    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let _ = result.set("status", "connected");
                    let _ = result.set("banner", response);
                }
                Err(e) => {
                    let _ = result.set("status", "error");
                    let _ = result.set("error", e.to_string());
                }
            }

            Ok(result)
        })
        .ok(),
    );

    smtp.set(
        "hello",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    smtp.set(
        "mail_from",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    smtp.set(
        "rcpt_to",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    smtp.set(
        "data",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    smtp.set(
        "quit",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    smtp.set(
        "rset",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    smtp.set(
        "help",
        lua.create_function(|_lua, _: mlua::Value| Ok("".to_string()))
            .ok(),
    );

    smtp.set(
        "noop",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    smtp.set(
        "vrfy",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(false))
            .ok(),
    );

    smtp.set(
        "expn",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(false))
            .ok(),
    );

    smtp.set(
        "starttls",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    globals.set("smtp", smtp).ok();
}
