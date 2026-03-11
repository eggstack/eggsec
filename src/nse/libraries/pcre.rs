//! NSE pcre library wrapper
//!
//! PCRE (Perl Compatible Regular Expressions) bindings for NSE scripts.
//! This wrapper uses the Rust regex crate under the hood for compatibility.

use mlua::{Lua, Result as LuaResult};
use std::collections::HashMap;
use std::sync::Mutex;

static COMPILED_REGEX: once_cell::sync::Lazy<Mutex<HashMap<usize, regex::Regex>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static REGEX_COUNTER: once_cell::sync::Lazy<Mutex<usize>> =
    once_cell::sync::Lazy::new(|| Mutex::new(1));

pub fn register_pcre_library(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();
    let pcre = lua.create_table()?;

    let match_fn = lua.create_function(
        |_lua, (pattern, subject, opts): (String, String, Option<String>)| {
            let case_insensitive = opts.as_ref().map_or(false, |o| o.contains('i'));

            let regex_pattern = if case_insensitive {
                format!("(?i){}", pattern)
            } else {
                pattern
            };

            match regex::Regex::new(&regex_pattern) {
                Ok(re) => {
                    let mut results = Vec::new();

                    for cap in re.captures_iter(&subject) {
                        let mut match_result = Vec::new();
                        for i in 0..cap.len() {
                            if let Some(m) = cap.get(i) {
                                match_result.push(m.as_str().to_string());
                            } else {
                                match_result.push(String::new());
                            }
                        }
                        results.push(match_result);
                    }

                    Ok(results)
                }
                Err(_e) => Ok(Vec::<Vec<String>>::new()),
            }
        },
    )?;
    pcre.set("match", match_fn)?;

    let match_one_fn = lua.create_function(
        |_lua, (pattern, subject, opts): (String, String, Option<String>)| {
            let case_insensitive = opts.as_ref().map_or(false, |o| o.contains('i'));

            let regex_pattern = if case_insensitive {
                format!("(?i){}", pattern)
            } else {
                pattern
            };

            match regex::Regex::new(&regex_pattern) {
                Ok(re) => {
                    if let Some(cap) = re.captures(&subject) {
                        let mut results = Vec::new();
                        for i in 0..cap.len() {
                            if let Some(m) = cap.get(i) {
                                results.push(m.as_str().to_string());
                            } else {
                                results.push(String::new());
                            }
                        }
                        Ok(results)
                    } else {
                        Ok(Vec::<String>::new())
                    }
                }
                Err(_) => Ok(Vec::<String>::new()),
            }
        },
    )?;
    pcre.set("match_one", match_one_fn)?;

    let compile_fn = lua.create_function(|_lua, (pattern, opts): (String, Option<String>)| {
        let case_insensitive = opts.as_ref().map_or(false, |o| o.contains('i'));

        let regex_pattern = if case_insensitive {
            format!("(?i){}", pattern)
        } else {
            pattern
        };

        match regex::Regex::new(&regex_pattern) {
            Ok(re) => {
                let mut counter = REGEX_COUNTER.lock().unwrap();
                let id = *counter;
                *counter += 1;

                COMPILED_REGEX.lock().unwrap().insert(id, re);

                Ok(id)
            }
            Err(e) => Err(mlua::Error::RuntimeError(format!("Invalid regex: {}", e))),
        }
    })?;
    pcre.set("compile", compile_fn)?;

    let exec_fn = lua.create_function(|_lua, (id, subject): (usize, String)| {
        let compiled = COMPILED_REGEX.lock().unwrap();

        if let Some(re) = compiled.get(&id) {
            if let Some(cap) = re.captures(&subject) {
                let mut results = Vec::new();
                for i in 0..cap.len() {
                    if let Some(m) = cap.get(i) {
                        results.push(m.as_str().to_string());
                    } else {
                        results.push(String::new());
                    }
                }
                return Ok(results);
            }
        }

        Ok(Vec::<String>::new())
    })?;
    pcre.set("exec", exec_fn)?;

    let free_fn = lua.create_function(|_lua, id: usize| {
        COMPILED_REGEX.lock().unwrap().remove(&id);
        Ok(true)
    })?;
    pcre.set("free", free_fn)?;

    let substitute_fn = lua.create_function(
        |_lua, (pattern, subject, replacement, opts): (String, String, String, Option<String>)| {
            let case_insensitive = opts.as_ref().map_or(false, |o| o.contains('i'));

            let regex_pattern = if case_insensitive {
                format!("(?i){}", pattern)
            } else {
                pattern
            };

            match regex::Regex::new(&regex_pattern) {
                Ok(re) => {
                    let result = re.replace_all(&subject, replacement.as_str()).to_string();
                    Ok(result)
                }
                Err(_) => Ok(subject),
            }
        },
    )?;
    pcre.set("substitute", substitute_fn)?;

    let split_fn = lua.create_function(
        |_lua, (pattern, subject, max): (String, String, Option<usize>)| {
            let regex_pattern = pattern;

            match regex::Regex::new(&regex_pattern) {
                Ok(re) => {
                    let parts: Vec<String> = re.split(&subject).map(|s| s.to_string()).collect();
                    let max = max.unwrap_or(parts.len());
                    Ok(parts.into_iter().take(max).collect())
                }
                Err(_) => Ok(vec![subject]),
            }
        },
    )?;
    pcre.set("split", split_fn)?;

    let quote_fn = lua.create_function(|_lua, s: String| Ok(regex::escape(&s)))?;
    pcre.set("quote", quote_fn)?;

    let version_fn = lua.create_function(|_lua, _: ()| Ok("8.45"))?;
    pcre.set("version", version_fn)?;

    globals.set("pcre", pcre)?;
    Ok(())
}
