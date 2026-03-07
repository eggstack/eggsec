//! NSE stdnse (Standard NSE Functions) library
//!
//! Provides standard utility functions that NSE scripts depend on.

use mlua::{Lua, Table, Value};

pub fn register_stdlib(lua: &Lua) {
    let globals = lua.globals();

    let stdnse = lua.create_table().expect("Failed to create stdnse table");

    stdnse.set("verbose", 1).ok();
    stdnse.set("silent", false).ok();
    stdnse.set("script_args", "").ok();
    stdnse
        .set(
            "args",
            lua.create_table().expect("Failed to create args table"),
        )
        .ok();
    stdnse.set("loglevel", "normal").ok();

    stdnse
        .set(
            "format_output",
            lua.create_function(|_lua, (output, options): (Table, Option<Table>)| {
                let separator = options
                    .and_then(|o| o.get::<String>("separator").ok())
                    .unwrap_or_else(|| "\n".to_string());

                let mut lines = Vec::new();
                let mut severity = "ok".to_string();

                for pair in output.pairs::<Value, Value>() {
                    if let Ok((k, v)) = pair {
                        let key = k.to_string();
                        let val = v.to_string();
                        if key == "severity" {
                            severity = val;
                        } else if key == "status" {
                            severity = val;
                        } else {
                            lines.push(format!("{}: {}", key, val));
                        }
                    }
                }

                let result = if lines.is_empty() {
                    String::new()
                } else {
                    lines.join(&separator)
                };
                Ok((result, severity))
            }),
        )
        .ok();

    stdnse
        .set(
            "output_table",
            lua.create_function(|lua, table: Option<Table>| match table {
                Some(t) => Ok(Value::Table(t)),
                None => Ok(Value::Table(lua.create_table()?)),
            }),
        )
        .ok();

    stdnse
        .set(
            "get_script_args",
            lua.create_function(|lua, key: Option<String>| {
                if let Some(key) = key {
                    let stdnse_table = lua
                        .globals()
                        .get::<Table>("stdnse")
                        .expect("Failed to get stdnse");
                    if let Ok(args_table) = stdnse_table.get::<Table>("args") {
                        if let Ok(val) = args_table.get::<String>(key.as_str()) {
                            return Ok(val);
                        }
                    }
                    Ok("".to_string())
                } else {
                    let stdnse_table = lua
                        .globals()
                        .get::<Table>("stdnse")
                        .expect("Failed to get stdnse");
                    stdnse_table
                        .get::<String>("script_args")
                        .or_else(|_| Ok("".to_string()))
                }
            }),
        )
        .ok();

    stdnse
        .set(
            "get_script_args_string",
            lua.create_function(|lua, _: ()| {
                let stdnse_table = lua
                    .globals()
                    .get::<Table>("stdnse")
                    .expect("Failed to get stdnse");
                stdnse_table
                    .get::<String>("script_args")
                    .or_else(|_| Ok("".to_string()))
            }),
        )
        .ok();

    stdnse
        .set(
            "debug",
            lua.create_function(|_lua, (level, message): (i32, String)| {
                tracing::debug!("[NSE:{}] {}", level, message);
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "print_debug",
            lua.create_function(|_lua, (level, message): (i32, String)| {
                tracing::debug!("[NSE:{}] {}", level, message);
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "info",
            lua.create_function(|_lua, (level, message): (i32, String)| {
                tracing::info!("[NSE:{}] {}", level, message);
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "print",
            lua.create_function(|_lua, message: String| {
                println!("{}", message);
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "print_verbose",
            lua.create_function(|_lua, message: String| {
                println!("[verbose] {}", message);
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "print_debug",
            lua.create_function(|_lua, (level, message): (i32, String)| {
                eprintln!("[DEBUG:{}] {}", level, message);
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "sleep",
            lua.create_function(|_lua, seconds: f64| {
                std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "make_output",
            lua.create_function(|lua, data: Option<Table>| match data {
                Some(t) => Ok(Value::Table(t)),
                None => Ok(Value::Table(lua.create_table()?)),
            }),
        )
        .ok();

    stdnse
        .set(
            "verbosity",
            lua.create_function(|lua, level: Option<i32>| {
                if let Some(l) = level {
                    let stdnse_table = lua.globals().get::<Table>("stdnse").ok();
                    if let Some(stdnse) = stdnse_table {
                        let _ = stdnse.set("verbose", l);
                    }
                }
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "get_verbosity",
            lua.create_function(|lua, _: ()| {
                let stdnse_table = lua
                    .globals()
                    .get::<Table>("stdnse")
                    .expect("Failed to get stdnse");
                stdnse_table.get::<i32>("verbose").unwrap_or(1)
            }),
        )
        .ok();

    stdnse
        .set(
            "silent",
            lua.create_function(|lua, silent: Option<bool>| {
                if let Some(s) = silent {
                    let stdnse_table = lua.globals().get::<Table>("stdnse").ok();
                    if let Some(stdnse) = stdnse_table {
                        let _ = stdnse.set("silent", s);
                    }
                }
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "get_silent",
            lua.create_function(|lua, _: ()| {
                let stdnse_table = lua
                    .globals()
                    .get::<Table>("stdnse")
                    .expect("Failed to get stdnse");
                stdnse_table.get::<bool>("silent").unwrap_or(false)
            }),
        )
        .ok();

    stdnse
        .set(
            "new_thread",
            lua.create_function(|lua, (func, args): (mlua::Function, Table)| {
                let thread = lua.create_thread(func)?;
                let _ = thread.resume(args);
                Ok(Value::Thread(thread))
            }),
        )
        .ok();

    stdnse
        .set(
            "start_clock",
            lua.create_function(|_lua, _: ()| {
                Ok(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64)
            }),
        )
        .ok();

    stdnse
        .set(
            "elapsed_time",
            lua.create_function(|_lua, start: i64| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                Ok(now - start)
            }),
        )
        .ok();

    stdnse
        .set(
            "strsplit",
            lua.create_function(|_lua, (delimiter, s, max): (String, String, Option<i32>)| {
                let parts: Vec<String> = if let Some(m) = max {
                    s.splitn(m as usize, &delimiter)
                        .map(|p| p.to_string())
                        .collect()
                } else {
                    s.split(&delimiter).map(|p| p.to_string()).collect()
                };
                Ok(parts)
            }),
        )
        .ok();

    stdnse
        .set(
            "join",
            lua.create_function(|_lua, (delimiter, parts): (String, Vec<String>)| {
                Ok(parts.join(&delimiter))
            }),
        )
        .ok();

    stdnse
        .set(
            "tohex",
            lua.create_function(|_lua, (s, options): (String, Option<Table>)| {
                let separator = options
                    .and_then(|o| o.get::<String>("separator").ok())
                    .unwrap_or_default();

                let hex: String = if separator.is_empty() {
                    s.bytes().map(|b| format!("{:02x}", b)).collect()
                } else {
                    s.bytes()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(&separator)
                };
                Ok(hex)
            }),
        )
        .ok();

    stdnse
        .set(
            "fromhex",
            lua.create_function(|_lua, s: String| {
                let mut bytes = Vec::new();
                let mut chars = s.chars().peekable();
                while let Some(c) = chars.next() {
                    if let Some(n) = chars.next() {
                        let hex = format!("{}{}", c, n);
                        if let Ok(b) = u8::from_str_radix(&hex, 16) {
                            bytes.push(b);
                        }
                    }
                }
                Ok(String::from_utf8_lossy(&bytes).to_string())
            }),
        )
        .ok();

    stdnse
        .set(
            "registry",
            lua.create_function(|lua, _: ()| {
                let registry = lua.create_table()?;
                Ok(registry)
            }),
        )
        .ok();

    stdnse
        .set(
            "get_registry",
            lua.create_function(|lua, _: ()| {
                let stdnse_table = lua.globals().get::<Table>("stdnse").ok();
                if let Some(stdnse) = stdnse_table {
                    if let Ok(reg) = stdnse.get::<Table>("registry") {
                        return Ok(reg);
                    }
                }
                let registry = lua.create_table()?;
                Ok(registry)
            }),
        )
        .ok();

    stdnse
        .set(
            "set_registry",
            lua.create_function(|lua, (key, value): (String, Value)| {
                let stdnse_table = lua.globals().get::<Table>("stdnse").ok();
                if let Some(stdnse) = stdnse_table {
                    let registry = stdnse
                        .get::<Table>("registry")
                        .unwrap_or_else(|_| lua.create_table().expect("Failed to create registry"));
                    let _ = registry.set(key, value);
                }
                Ok(())
            }),
        )
        .ok();

    stdnse
        .set(
            "base64",
            lua.create_function(|_lua, s: String| {
                use base64::Engine;
                Ok(base64::engine::general_purpose::STANDARD.encode(s))
            }),
        )
        .ok();

    stdnse
        .set(
            "base64_decode",
            lua.create_function(|_lua, s: String| {
                use base64::Engine;
                match base64::engine::general_purpose::STANDARD.decode(&s) {
                    Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).to_string()),
                    Err(_) => Ok("".to_string()),
                }
            }),
        )
        .ok();

    stdnse
        .set(
            "unique",
            lua.create_function(|_lua, items: Vec<String>| {
                let mut unique: Vec<String> = items.into_iter().collect();
                unique.sort();
                unique.dedup();
                Ok(unique)
            }),
        )
        .ok();

    stdnse
        .set(
            "keys",
            lua.create_function(|lua, table: Table| {
                let keys = lua.create_table()?;
                for (i, pair) in table.pairs::<Value, Value>().enumerate() {
                    if let Ok((k, _)) = pair {
                        let _ = keys.set(i + 1, k);
                    }
                }
                Ok(keys)
            }),
        )
        .ok();

    stdnse
        .set(
            "sorted_keys",
            lua.create_function(|lua, table: Table| {
                let mut keys: Vec<String> = Vec::new();
                for pair in table.pairs::<Value, Value>() {
                    if let Ok((k, _)) = pair {
                        keys.push(k.to_string());
                    }
                }
                keys.sort();
                let sorted = lua.create_table()?;
                for (i, k) in keys.iter().enumerate() {
                    let _ = sorted.set(i + 1, k.clone());
                }
                Ok(sorted)
            }),
        )
        .ok();

    stdnse
        .set("version", lua.create_function(|_lua, _: ()| Ok("1.0.0")))
        .ok();

    globals.set("stdnse", stdnse).ok();
}
