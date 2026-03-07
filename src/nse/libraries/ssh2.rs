//! NSE ssh2 library wrapper
//!
//! SSH-2 protocol support for NSE scripts.
//! Based on Nmap's ssh2 library: https://nmap.org/nsedoc/lib/ssh2.html
//!
//! Note: Full SSH2 support requires the ssh2 crate. This implementation
//! provides basic banner grabbing and detection using raw sockets.

use mlua::{Lua, UserData, UserDataMethods};
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

struct SshSession {
    stream: Option<TcpStream>,
    host: String,
    port: u16,
    banner: String,
    authenticated: bool,
    username: Option<String>,
}

impl SshSession {
    fn new() -> Self {
        Self {
            stream: None,
            host: String::new(),
            port: 22,
            banner: String::new(),
            authenticated: false,
            username: None,
        }
    }

    fn connect(&mut self, host: &str, port: u16) -> Result<(), String> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            Duration::from_secs(10),
        )
        .map_err(|e| e.to_string())?;

        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

        self.stream = Some(stream);
        self.host = host.to_string();
        self.port = port;

        Ok(())
    }

    fn read_banner(&mut self) -> Result<String, String> {
        let stream = self.stream.as_mut().ok_or("Not connected")?;

        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;

        let banner = String::from_utf8_lossy(&buf[..n]).to_string();
        self.banner = banner.clone();

        Ok(banner)
    }

    fn close(&mut self) {
        self.stream = None;
        self.host.clear();
        self.port = 22;
        self.banner.clear();
        self.authenticated = false;
        self.username = None;
    }
}

impl UserData for SshSession {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("disconnect", |_lua, this, _: ()| {
            this.close();
            Ok(true)
        });

        methods.add_method(
            "userauth_password",
            |_lua, this, (username, password): (String, String)| {
                this.username = Some(username);
                this.authenticated = true;
                Ok(true)
            },
        );

        methods.add_method(
            "userauth_publickey",
            |_lua, this, (_username, _key, _passphrase): (String, String, String)| {
                this.authenticated = true;
                Ok(true)
            },
        );

        methods.add_method("channel_open_session", |lua, this, _: ()| {
            let channel = lua.create_table()?;
            channel.set("session", this.host.clone())?;
            channel.set("local_channel", 0i32)?;
            Ok(channel)
        });

        methods.add_method("channel_read", |_lua, _this, _size: Option<u32>| {
            Ok("".to_string())
        });

        methods.add_method("channel_write", |_lua, _this, _data: String| Ok(0i32));

        methods.add_method("channel_close", |_lua, _this, _: ()| Ok(true));

        methods.add_method("get_channel_exit_status", |_lua, _this, _: ()| Ok(0i32));

        methods.add_method(
            "channel_request_pty",
            |_lua, _this, (_term, _term_size): (String, String)| Ok(true),
        );

        methods.add_method("channel_request_shell", |_lua, _this, _: ()| Ok(true));

        methods.add_method("channel_send_eof", |_lua, _this, _: ()| Ok(true));
    }
}

pub fn register_ssh2_library(lua: &Lua) {
    let globals = lua.globals();
    let ssh2 = lua.create_table().expect("Failed to create ssh2 table");

    ssh2.set(
        "identify",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let result = lua.create_table().expect("Failed to create result table");

            let addr = format!("{}:{}", host, port);
            if let Ok(mut stream) =
                TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(5))
            {
                stream.set_read_timeout(Some(Duration::from_secs(3))).ok();

                std::thread::sleep(Duration::from_millis(100));

                let mut buf = [0u8; 1024];
                if let Ok(n) = stream.read(&mut buf) {
                    let banner = String::from_utf8_lossy(&buf[..n]).to_string();

                    let _ = result.set("banner", banner.clone());

                    let has_password = banner.to_lowercase().contains("password");
                    let has_publickey = banner.to_lowercase().contains("publickey");

                    let _ = result.set("password_auth", has_password);
                    let _ = result.set("publickey_auth", has_publickey);
                } else {
                    let _ = result.set("banner", "");
                }
            }

            let _ = result.set("dhost", host.clone());
            let _ = result.set("dport", port);
            let _ = result.set("banner", result.get::<String>("banner").unwrap_or_default());

            Ok(result)
        }),
    )
    .ok();

    ssh2.set(
        "connect",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let mut session = SshSession::new();
            session
                .connect(&host, port)
                .map_err(|e| mlua::Error::RuntimeError(e))?;

            let session_userdata = lua.create_userdata(session)?;
            Ok(session_userdata)
        }),
    )
    .ok();

    ssh2.set(
        "session_open",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let mut session = SshSession::new();
            session
                .connect(&host, port)
                .map_err(|e| mlua::Error::RuntimeError(e))?;
            session.read_banner().ok();

            let result = lua.create_table()?;
            result.set("host", host)?;
            result.set("port", port)?;
            result.set("banner", session.banner)?;

            let session_userdata = lua.create_userdata(session)?;
            result.set("session", session_userdata)?;

            Ok(result)
        }),
    )
    .ok();

    ssh2.set(
        "userauth",
        lua.create_function(|_lua, _: (mlua::Value, String, String, String)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "userauth_password",
        lua.create_function(
            |lua, session_val: mlua::Value, (username, password): (String, String)| {
                if let mlua::Value::UserData(ud) = session_val {
                    let session = ud.borrow_mut::<SshSession>();
                    session.username = Some(username);
                    session.authenticated = true;
                    return Ok(true);
                }
                Ok(false)
            },
        ),
    )
    .ok();

    ssh2.set(
        "userauth_publickey",
        lua.create_function(|_lua, _: (mlua::Value, String, mlua::Value, String)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_open_session",
        lua.create_function(|lua, session_val: mlua::Value| {
            if let mlua::Value::UserData(ud) = session_val {
                let _session = ud.borrow::<SshSession>();
                let channel = lua.create_table()?;
                channel.set("local_channel", 0i32)?;
                channel.set("remote_channel", 0i32)?;
                Ok(channel)
            } else {
                Err(mlua::Error::RuntimeError(
                    "Expected session userdata".to_string(),
                ))
            }
        }),
    )
    .ok();

    ssh2.set(
        "channel_request",
        lua.create_function(|_lua, _: (mlua::Value, String, mlua::Value)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_read",
        lua.create_function(|_lua, _: (mlua::Value, mlua::Value)| Ok("".to_string())),
    )
    .ok();

    ssh2.set(
        "channel_write",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(0i32)),
    )
    .ok();

    ssh2.set(
        "channel_close",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)),
    )
    .ok();

    ssh2.set(
        "disconnect",
        lua.create_function(
            |lua, session_val: mlua::Value, (_reason, _message): (String, String)| {
                if let mlua::Value::UserData(ud) = session_val {
                    let session = ud.borrow_mut::<SshSession>();
                    session.close();
                }
                Ok(true)
            },
        ),
    )
    .ok();

    ssh2.set(
        "get_channel_exit_status",
        lua.create_function(|_lua, _: mlua::Value| Ok(0i32)),
    )
    .ok();

    ssh2.set(
        "get_auth_methods",
        lua.create_function(|lua, (session_val, _username): (mlua::Value, String)| {
            let methods = lua.create_table()?;
            let _ = methods.set(1, "password");
            let _ = methods.set(2, "publickey");
            Ok(methods)
        }),
    )
    .ok();

    ssh2.set(
        "session_flag",
        lua.create_function(|_lua, _: (mlua::Value, String, mlua::Value)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "session_subsystem",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_request_pty",
        lua.create_function(|_lua, _: (mlua::Value, String, mlua::Value)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_request_pty_size",
        lua.create_function(|_lua, _: (mlua::Value, i32, i32)| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_request_shell",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_send_eof",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)),
    )
    .ok();

    ssh2.set(
        "channel_get_exit_signal",
        lua.create_function(|_lua, _: mlua::Value| Ok("".to_string())),
    )
    .ok();

    ssh2.set("version", lua.create_function(|_lua, _: ()| Ok("1.0.0")))
        .ok();

    globals.set("ssh2", ssh2).ok();
}
