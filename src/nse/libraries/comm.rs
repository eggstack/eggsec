//! NSE comm library wrapper
//!
//! Provides low-level socket communication for banner grabbing and data exchange.

use mlua::{Lua, Table};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn register_comm_library(lua: &Lua) {
    let globals = lua.globals();

    let comm = lua.create_table().expect("Failed to create comm table");

    // comm.get_banner function
    comm.set(
        "get_banner",
        lua.create_function(
            |lua, (host, port, _options): (String, u16, Option<Table>)| {
                let timeout = Duration::from_secs(5);
                let addr = format!("{}:{}", host, port);

                match addr.parse() {
                    Ok(socket_addr) => {
                        match TcpStream::connect_timeout(&socket_addr, timeout) {
                            Ok(mut stream) => {
                                stream.set_read_timeout(Some(timeout)).ok();

                                // Wait briefly for banner
                                std::thread::sleep(Duration::from_millis(500));

                                let mut buf = vec![0u8; 4096];
                                match stream.read(&mut buf) {
                                    Ok(n) => {
                                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                                        let result = lua.create_table()?;
                                        result.set("data", data)?;
                                        Ok(result)
                                    }
                                    Err(_) => {
                                        let result = lua.create_table()?;
                                        result.set("data", "")?;
                                        Ok(result)
                                    }
                                }
                            }
                            Err(_) => {
                                let result = lua.create_table()?;
                                result.set("data", "")?;
                                Ok(result)
                            }
                        }
                    }
                    Err(_) => {
                        let result = lua.create_table()?;
                        result.set("data", "")?;
                        Ok(result)
                    }
                }
            },
        )
        .ok(),
    );

    // comm.exchange function
    comm.set(
        "exchange",
        lua.create_function(
            |lua, (host, port, data, _options): (String, u16, String, Option<Table>)| {
                let timeout = Duration::from_secs(5);
                let addr = format!("{}:{}", host, port);

                match addr.parse() {
                    Ok(socket_addr) => {
                        match TcpStream::connect_timeout(&socket_addr, timeout) {
                            Ok(mut stream) => {
                                stream.set_read_timeout(Some(timeout)).ok();
                                stream.set_write_timeout(Some(timeout)).ok();

                                // Send data
                                if let Err(_) = stream.write_all(data.as_bytes()) {
                                    let result = lua.create_table()?;
                                    result.set("data", "")?;
                                    return Ok(result);
                                }

                                // Wait for response
                                std::thread::sleep(Duration::from_millis(500));

                                let mut buf = vec![0u8; 4096];
                                match stream.read(&mut buf) {
                                    Ok(n) => {
                                        let response =
                                            String::from_utf8_lossy(&buf[..n]).to_string();
                                        let result = lua.create_table()?;
                                        result.set("data", response)?;
                                        Ok(result)
                                    }
                                    Err(_) => {
                                        let result = lua.create_table()?;
                                        result.set("data", "")?;
                                        Ok(result)
                                    }
                                }
                            }
                            Err(_) => {
                                let result = lua.create_table()?;
                                result.set("data", "")?;
                                Ok(result)
                            }
                        }
                    }
                    Err(_) => {
                        let result = lua.create_table()?;
                        result.set("data", "")?;
                        Ok(result)
                    }
                }
            },
        )
        .ok(),
    );

    // comm.tryssl function
    comm.set(
        "tryssl",
        lua.create_function(
            |lua, (host, port, _data, _options): (String, u16, String, Option<Table>)| {
                let url = format!("https://{}:{}", host, port);

                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .danger_accept_invalid_certs(true)
                    .build();

                match client {
                    Ok(c) => match c.get(&url).send() {
                        Ok(resp) => {
                            let status = resp.status().as_u16();
                            let body = resp.text().unwrap_or_default();

                            let result = lua.create_table()?;
                            result.set("status", status as i32)?;
                            result.set("data", body)?;
                            Ok(result)
                        }
                        Err(e) => {
                            let result = lua.create_table()?;
                            result.set("status", 0i32)?;
                            result.set("data", e.to_string())?;
                            Ok(result)
                        }
                    },
                    Err(e) => {
                        let result = lua.create_table()?;
                        result.set("status", 0i32)?;
                        result.set("data", e.to_string())?;
                        Ok(result)
                    }
                }
            },
        )
        .ok(),
    );

    // comm.close function
    comm.set(
        "close",
        lua.create_function(|_, _socket: Table| Ok(())).ok(),
    );

    globals.set("comm", comm).ok();
}
