//! NSE nmap library wrapper
//!
//! Provides access to Nmap internals like host info, ports, and socket operations.

use mlua::Lua;

pub fn register_nmap_library(lua: &Lua) {
    let globals = lua.globals();

    let nmap = lua.create_table().expect("Failed to create nmap table");

    nmap.set("target", "").ok();
    nmap.set("address_family", "inet").ok();
    nmap.set("version", "1.0.0").ok();
    nmap.set("numopen", 0i32).ok();

    nmap.set(
        "get_hostname",
        lua.create_function(|_lua, host: Option<String>| Ok(host.unwrap_or_default()))
            .ok(),
    );

    nmap.set(
        "get_port_state",
        lua.create_function(|lua, (_host, port): (Option<String>, u16)| {
            let t = lua.create_table()?;
            t.set("number", port)?;
            t.set("protocol", "tcp")?;
            t.set("state", "unknown")?;
            Ok(t)
        })
        .ok(),
    );

    nmap.set(
        "get_ports",
        lua.create_function(
            |lua, _: (Option<String>, Option<u16>, Option<String>, Option<String>)| {
                Ok(lua.create_table()?)
            },
        )
        .ok(),
    );

    nmap.set(
        "set_port_state",
        lua.create_function(|_lua, _: (Option<String>, u16, String)| Ok(()))
            .ok(),
    );

    nmap.set(
        "new_socket",
        lua.create_function(|lua, _: (Option<String>, Option<String>)| {
            let socket = lua.create_table()?;
            socket.set("closed", false)?;
            Ok(socket)
        })
        .ok(),
    );

    nmap.set(
        "registry",
        lua.create_function(|lua, _: ()| Ok(lua.create_table()?))
            .ok(),
    );

    nmap.set(
        "ref_increment",
        lua.create_function(|_lua, _: ()| Ok(())).ok(),
    );
    nmap.set(
        "ref_decrement",
        lua.create_function(|_lua, _: ()| Ok(())).ok(),
    );

    nmap.set(
        "is_admin",
        lua.create_function(|_lua, _: ()| {
            #[cfg(unix)]
            {
                Ok(std::process::Command::new("id")
                    .arg("-u")
                    .output()
                    .map(|o| o.stdout == b"0\n")
                    .unwrap_or(false))
            }
            #[cfg(not(unix))]
            {
                Ok(false)
            }
        })
        .ok(),
    );

    nmap.set(
        "current_time",
        lua.create_function(|_lua, _: ()| Ok(chrono::Utc::now().timestamp()))
            .ok(),
    );

    nmap.set(
        "get_random_bytes",
        lua.create_function(|_lua, count: i32| {
            let bytes: Vec<u8> = (0..count.max(0) as usize)
                .map(|_| rand::random::<u8>())
                .collect();
            Ok(bytes)
        })
        .ok(),
    );

    nmap.set(
        "get_random",
        lua.create_function(|_lua, (min, max): (i32, i32)| {
            if min >= max {
                return Ok(min);
            }
            Ok(rand::random::<i32>() % (max - min + 1) + min)
        })
        .ok(),
    );

    globals.set("nmap", nmap).ok();
}
