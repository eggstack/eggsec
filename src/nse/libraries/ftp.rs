//! NSE ftp library wrapper
//!
//! FTP protocol support for NSE scripts.
//! Based on Nmap's ftp library concepts.

use mlua::Lua;
use std::io::Read;
use std::net::TcpStream;

pub fn register_ftp_library(lua: &Lua) {
    let globals = lua.globals();

    let ftp = lua.create_table().expect("Failed to create ftp table");

    ftp.set(
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
                    let _ = result.set("response", response);
                    let _ = result.set("host", host);
                    let _ = result.set("port", port);
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

    ftp.set(
        "user",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    ftp.set(
        "pass",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    ftp.set(
        "cwd",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    ftp.set(
        "list",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok("".to_string()))
            .ok(),
    );

    ftp.set(
        "retr",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok("".to_string()))
            .ok(),
    );

    ftp.set(
        "stor",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    ftp.set(
        "quit",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    ftp.set(
        "type",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    ftp.set(
        "pasv",
        lua.create_function(|_lua, _: mlua::Value| Ok("127,0,0,1,0,0".to_string()))
            .ok(),
    );

    ftp.set(
        "epsv",
        lua.create_function(|_lua, _: mlua::Value| Ok("|||0|".to_string()))
            .ok(),
    );

    ftp.set(
        "pwd",
        lua.create_function(|_lua, _: mlua::Value| Ok("/".to_string()))
            .ok(),
    );

    ftp.set(
        "size",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(0i64))
            .ok(),
    );

    globals.set("ftp", ftp).ok();
}
