//! NSE socket library wrapper
//!
//! Low-level socket operations for NSE scripts.
//! Based on Nmap's socket library concepts.

use mlua::{Lua, UserData, UserDataMethods, Value};
use std::io::Read;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

struct SocketHandle {
    stream: Option<TcpStream>,
    host: String,
    port: u16,
    timeout: Duration,
}

impl SocketHandle {
    fn new() -> Self {
        Self {
            stream: None,
            host: String::new(),
            port: 0,
            timeout: Duration::from_secs(10),
        }
    }

    fn connect(&mut self, host: &str, port: u16) -> Result<(), String> {
        let addr = format!("{}:{}", host, port);
        let socket_addr: SocketAddr = addr
            .to_socket_addrs()
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("No address found")?;

        let stream =
            TcpStream::connect_timeout(&socket_addr, self.timeout).map_err(|e| e.to_string())?;

        stream.set_read_timeout(Some(self.timeout)).ok();
        stream.set_write_timeout(Some(self.timeout)).ok();

        self.stream = Some(stream);
        self.host = host.to_string();
        self.port = port;

        Ok(())
    }

    fn send(&mut self, data: &str) -> Result<usize, String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;
        stream.write(data.as_bytes()).map_err(|e| e.to_string())
    }

    fn receive(&mut self, size: usize) -> Result<String, String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;
        let size = size.max(1).min(65536);
        let mut buffer = vec![0u8; size];
        let n = stream.read(&mut buffer).map_err(|e| e.to_string())?;
        buffer.truncate(n);
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    fn close(&mut self) {
        self.stream = None;
        self.host.clear();
        self.port = 0;
    }

    fn set_timeout(&mut self, timeout_ms: i64) {
        self.timeout = Duration::from_millis(timeout_ms.max(0) as u64);
        if let Some(ref mut stream) = self.stream {
            stream.set_read_timeout(Some(self.timeout)).ok();
            stream.set_write_timeout(Some(self.timeout)).ok();
        }
    }
}

impl UserData for SocketHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("connect", |lua, this, (host, port): (String, u16)| {
            this.connect(&host, port)
                .map_err(|e| mlua::Error::RuntimeError(e))?;

            let result = lua.create_table()?;
            result.set("host", host)?;
            result.set("port", port)?;
            result.set("status", "connected")?;
            Ok(result)
        });

        methods.add_method("send", |lua, this, data: String| {
            let bytes = this.send(&data).map_err(|e| mlua::Error::RuntimeError(e))?;

            let result = lua.create_table()?;
            result.set("status", "sent")?;
            result.set("bytes", bytes as i32)?;
            Ok(result)
        });

        methods.add_method("receive", |lua, this, size: Option<usize>| {
            let data = this
                .receive(size.unwrap_or(4096))
                .map_err(|e| mlua::Error::RuntimeError(e))?;

            let result = lua.create_table()?;
            result.set("status", "received")?;
            result.set("data", data)?;
            result.set("bytes", data.len() as i32)?;
            Ok(result)
        });

        methods.add_method("close", |_lua, this, _: ()| {
            this.close();
            Ok(true)
        });

        methods.add_method("set_timeout", |_lua, this, timeout_ms: i64| {
            this.set_timeout(timeout_ms);
            Ok(true)
        });

        methods.add_method("get_timeout", |_lua, this, _: ()| {
            Ok(this.timeout.as_millis() as i64)
        });

        methods.add_method("get_peer_name", |_lua, this, _: ()| {
            if let Some(ref stream) = this.stream {
                if let Ok(peer) = stream.peer_addr() {
                    return Ok(format!("{}:{}", peer.ip(), peer.port()));
                }
            }
            Ok("".to_string())
        });

        methods.add_method("is_connected", |_lua, this, _: ()| {
            Ok(this.stream.is_some())
        });
    }
}

pub fn register_socket_library(lua: &Lua) {
    let globals = lua.globals();

    let socket = lua.create_table().expect("Failed to create socket table");

    socket
        .set(
            "tcp",
            lua.create_function(|lua, _: ()| {
                let sock = SocketHandle::new();
                Ok(lua.create_userdata(sock)?)
            }),
        )
        .ok();

    socket
        .set(
            "tcp_connect",
            lua.create_function(|lua, (host, port): (String, u16)| {
                let mut sock = SocketHandle::new();
                sock.connect(&host, port)
                    .map_err(|e| mlua::Error::RuntimeError(e))?;

                let result = lua.create_table()?;
                result.set("host", host)?;
                result.set("port", port)?;
                result.set("status", "connected")?;
                Ok(result)
            }),
        )
        .ok();

    socket
        .set(
            "connect",
            lua.create_function(|lua, (host, port): (String, u16)| {
                let result = lua.create_table().expect("Failed to create result table");

                let addr = format!("{}:{}", host, port);
                match addr.to_socket_addrs() {
                    Ok(mut addrs) => {
                        if let Some(socket_addr) = addrs.next() {
                            match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
                            {
                                Ok(stream) => {
                                    let _ = result.set("status", "connected");
                                    let _ = result.set("host", host);
                                    let _ = result.set("port", port);
                                    drop(stream);
                                }
                                Err(e) => {
                                    let _ = result.set("status", "error");
                                    let _ = result.set("error", e.to_string());
                                }
                            }
                        } else {
                            let _ = result.set("status", "error");
                            let _ = result.set("error", "No address found");
                        }
                    }
                    Err(e) => {
                        let _ = result.set("status", "error");
                        let _ = result.set("error", e.to_string());
                    }
                }

                Ok(result)
            }),
        )
        .ok();

    socket
        .set(
            "send",
            lua.create_function(|lua, (host, port, data): (String, u16, String)| {
                let result = lua.create_table().expect("Failed to create result table");

                let addr = format!("{}:{}", host, port);
                let mut stream = match TcpStream::connect_timeout(
                    &addr
                        .to_socket_addrs()
                        .ok()
                        .and_then(|mut a| a.next())
                        .ok_or("No addr")?,
                    Duration::from_secs(10),
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = result.set("status", "error");
                        let _ = result.set("error", e.to_string());
                        return Ok(result);
                    }
                };

                stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

                match stream.write(data.as_bytes()) {
                    Ok(n) => {
                        let _ = result.set("status", "sent");
                        let _ = result.set("bytes", n as i32);
                    }
                    Err(e) => {
                        let _ = result.set("status", "error");
                        let _ = result.set("error", e.to_string());
                    }
                }

                Ok(result)
            }),
        )
        .ok();

    socket
        .set(
            "receive",
            lua.create_function(|lua, (host, port, size): (String, u16, usize)| {
                let result = lua.create_table().expect("Failed to create result table");

                let addr = format!("{}:{}", host, port);
                let mut stream = match TcpStream::connect_timeout(
                    &addr
                        .to_socket_addrs()
                        .ok()
                        .and_then(|mut a| a.next())
                        .ok_or("No addr")?,
                    Duration::from_secs(10),
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = result.set("status", "error");
                        let _ = result.set("error", e.to_string());
                        return Ok(result);
                    }
                };

                stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

                let mut buffer = vec![0u8; size.max(1).min(65536)];
                match stream.read(&mut buffer) {
                    Ok(n) => {
                        buffer.truncate(n);
                        let _ = result.set("status", "received");
                        let _ = result.set("data", String::from_utf8_lossy(&buffer).to_string());
                        let _ = result.set("bytes", n as i32);
                    }
                    Err(e) => {
                        let _ = result.set("status", "error");
                        let _ = result.set("error", e.to_string());
                    }
                }

                Ok(result)
            }),
        )
        .ok();

    socket
        .set(
            "receive_buf",
            lua.create_function(|lua, (socket_val, size): (Value, usize)| {
                if let Value::UserData(ud) = socket_val {
                    let sock = ud.borrow_mut::<SocketHandle>();
                    let data = sock
                        .receive(size)
                        .map_err(|e| mlua::Error::RuntimeError(e))?;

                    let result = lua.create_table()?;
                    result.set("status", "received")?;
                    result.set("data", data)?;
                    result.set("bytes", data.len() as i32)?;
                    Ok(result)
                } else {
                    Err(mlua::Error::RuntimeError(
                        "Expected socket userdata".to_string(),
                    ))
                }
            }),
        )
        .ok();

    socket
        .set(
            "send_buf",
            lua.create_function(|lua, (socket_val, data): (Value, String)| {
                if let Value::UserData(ud) = socket_val {
                    let sock = ud.borrow_mut::<SocketHandle>();
                    let bytes = sock.send(&data).map_err(|e| mlua::Error::RuntimeError(e))?;

                    let result = lua.create_table()?;
                    result.set("status", "sent")?;
                    result.set("bytes", bytes as i32)?;
                    Ok(result)
                } else {
                    Err(mlua::Error::RuntimeError(
                        "Expected socket userdata".to_string(),
                    ))
                }
            }),
        )
        .ok();

    socket
        .set(
            "close",
            lua.create_function(|lua, socket_val: Value| {
                if let Value::UserData(ud) = socket_val {
                    let sock = ud.borrow_mut::<SocketHandle>();
                    sock.close();
                    Ok(true)
                } else {
                    Ok(true)
                }
            }),
        )
        .ok();

    socket
        .set(
            "set_timeout",
            lua.create_function(|_lua, (socket_val, timeout_ms): (Value, i64)| {
                if let Value::UserData(ud) = socket_val {
                    let sock = ud.borrow_mut::<SocketHandle>();
                    sock.set_timeout(timeout_ms);
                }
                Ok(true)
            }),
        )
        .ok();

    socket
        .set(
            "get_timeout",
            lua.create_function(|_lua, socket_val: Value| -> Result<i64, mlua::Error> {
                if let Value::UserData(ud) = socket_val {
                    let sock = ud.borrow::<SocketHandle>();
                    return Ok(sock.timeout.as_millis() as i64);
                }
                Ok(10000)
            }),
        )
        .ok();

    socket
        .set("version", lua.create_function(|_lua, _: ()| Ok("1.0.0")))
        .ok();

    globals.set("socket", socket).ok();
}
