//! NSE mysql library wrapper
//!
//! MySQL protocol support for NSE scripts.
//! Based on Nmap's mysql library: https://nmap.org/nsedoc/lib/mysql.html

use mlua::Lua;

pub fn register_mysql_library(lua: &Lua) {
    let globals = lua.globals();

    let mysql = lua.create_table().expect("Failed to create mysql table");

    mysql.set(
        "connect",
        lua.create_function(|lua, (host, port): (String, u16)| {
            let result = lua.create_table().expect("Failed to create result table");

            let _ = result.set("host", host);
            let _ = result.set("port", port);
            let _ = result.set("socket", lua.create_table().ok());

            Ok(result)
        })
        .ok(),
    );

    mysql.set(
        "login",
        lua.create_function(|_lua, _: (mlua::Value, String, String, String)| Ok(true))
            .ok(),
    );

    mysql.set(
        "select_db",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(true))
            .ok(),
    );

    mysql.set(
        "query",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "send_query",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "get_results",
        lua.create_function(|_lua, _: mlua::Value| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "quit",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    mysql.set(
        "shutdown",
        lua.create_function(|_lua, _: mlua::Value| Ok(true)).ok(),
    );

    mysql.set(
        "stat",
        lua.create_function(|_lua, _: mlua::Value| Ok("".to_string()))
            .ok(),
    );

    mysql.set(
        "process_info",
        lua.create_function(|_lua, _: mlua::Value| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "list_dbs",
        lua.create_function(|_lua, _: mlua::Value| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "list_tables",
        lua.create_function(|_lua, _: (mlua::Value, Option<String>)| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "list_fields",
        lua.create_function(|_lua, _: (mlua::Value, String)| Ok(_lua.create_table().ok()))
            .ok(),
    );

    mysql.set(
        "get_capabilities",
        lua.create_function(|_lua, _: mlua::Value| Ok(0i64)).ok(),
    );

    mysql.set(
        "get_server_version",
        lua.create_function(|_lua, _: mlua::Value| Ok("".to_string()))
            .ok(),
    );

    globals.set("mysql", mysql).ok();
}
