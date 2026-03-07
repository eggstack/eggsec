//! NSE table library wrapper
//!
//! Table manipulation utilities for NSE scripts.
//! Based on Lua's table library extensions.

use mlua::{Lua, Table};

pub fn register_table_library(lua: &Lua) {
    let globals = lua.globals();

    let table_lib = lua.create_table().expect("Failed to create table table");

    table_lib.set(
        "serialize",
        lua.create_function(|_lua, (t, _options): (Table, Option<Table>)| {
            let mut output = Vec::new();

            fn serialize_value(output: &mut Vec<String>, value: &mlua::Value, indent: usize) {
                match value {
                    mlua::Value::String(s) => {
                        output.push(format!(
                            "\"{}\"",
                            s.to_string_lossy()
                                .replace('\\', "\\\\")
                                .replace('"', "\\\"")
                        ));
                    }
                    mlua::Value::Integer(i) => {
                        output.push(i.to_string());
                    }
                    mlua::Value::Number(n) => {
                        output.push(n.to_string());
                    }
                    mlua::Value::Boolean(b) => {
                        output.push(if *b {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        });
                    }
                    mlua::Value::Table(t) => {
                        output.push("{".to_string());
                        let mut first = true;
                        for pair in t.pairs::<mlua::Value, mlua::Value>() {
                            if let Ok((k, v)) = pair {
                                if !first {
                                    output.push(",".to_string());
                                }
                                first = false;
                                serialize_value(output, &k, indent + 1);
                                output.push(" = ".to_string());
                                serialize_value(output, &v, indent + 1);
                            }
                        }
                        output.push("}".to_string());
                    }
                    mlua::Value::Nil => {
                        output.push("nil".to_string());
                    }
                    _ => {
                        output.push("nil".to_string());
                    }
                }
            }

            serialize_value(&mut output, &mlua::Value::Table(t), 0);
            Ok(output.join(""))
        })
        .ok(),
    );

    table_lib.set(
        "deserialize",
        lua.create_function(|_lua, s: String| Ok(_lua.create_table().ok()))
            .ok(),
    );

    table_lib.set(
        "keys",
        lua.create_function(|_lua, t: Table| {
            let keys = _lua.create_table()?;
            let mut count = 0;
            for pair in t.pairs::<mlua::Value, mlua::Value>() {
                if let Ok((k, _)) = pair {
                    count += 1;
                    let _ = keys.set(count, k);
                }
            }
            Ok(keys)
        })
        .ok(),
    );

    table_lib.set(
        "values",
        lua.create_function(|_lua, t: Table| {
            let values = _lua.create_table()?;
            let mut count = 0;
            for pair in t.pairs::<mlua::Value, mlua::Value>() {
                if let Ok((_, v)) = pair {
                    count += 1;
                    let _ = values.set(count, v);
                }
            }
            Ok(values)
        })
        .ok(),
    );

    table_lib.set(
        "size",
        lua.create_function(|_lua, t: Table| {
            let mut count = 0;
            for _ in t.pairs::<mlua::Value, mlua::Value>() {
                count += 1;
            }
            Ok(count)
        })
        .ok(),
    );

    table_lib.set(
        "contains",
        lua.create_function(|_lua, (t, value): (Table, mlua::Value)| {
            for pair in t.pairs::<mlua::Value, mlua::Value>() {
                if let Ok((_, v)) = pair {
                    if v == value {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        })
        .ok(),
    );

    table_lib.set(
        "merge",
        lua.create_function(|_lua, (t1, t2): (Table, Table)| {
            let result = _lua.create_table()?;

            for pair in t1.pairs::<mlua::Value, mlua::Value>() {
                if let Ok((k, v)) = pair {
                    let _ = result.set(k, v);
                }
            }

            for pair in t2.pairs::<mlua::Value, mlua::Value>() {
                if let Ok((k, v)) = pair {
                    let _ = result.set(k, v);
                }
            }

            Ok(result)
        })
        .ok(),
    );

    globals.set("table", table_lib).ok();
}
