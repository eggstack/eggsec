//! NSE Executor - Lua VM setup and script execution
//!
//! This module provides the core functionality for running NSE scripts
//! using the mlua Lua interpreter (Lua 5.4).

use mlua::{Lua, Result as LuaResult, Table};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct NseExecutor {
    lua: Lua,
    target: String,
    scripts_path: Arc<Mutex<Vec<PathBuf>>>,
    output: Mutex<Vec<String>>,
}

impl NseExecutor {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        
        let scripts_path = Arc::new(Mutex::new(vec![]));
        
        let output = Mutex::new(vec![]);
        
        let executor = Self {
            lua,
            target: String::new(),
            scripts_path: scripts_path.clone(),
            output,
        };
        
        executor.register_libraries()?;
        executor.setup_require(scripts_path.clone())?;
        
        Ok(executor)
    }
    
    fn register_libraries(&self) -> LuaResult<()> {
        crate::nse::libraries::stdnse::register_stdlib(&self.lua);
        crate::nse::libraries::nmap::register_nmap_library(&self.lua);
        crate::nse::libraries::http::register_http_library(&self.lua);
        crate::nse::libraries::comm::register_comm_library(&self.lua);
        crate::nse::libraries::sslcert::register_sslcert_library(&self.lua);
        crate::nse::libraries::tls::register_tls_library(&self.lua);
        crate::nse::libraries::shortport::register_shortport_library(&self.lua);
        crate::nse::libraries::nse_string::register_string_library(&self.lua);
        crate::nse::libraries::nse_table::register_table_library(&self.lua);
        crate::nse::libraries::datetime::register_datetime_library(&self.lua);
        crate::nse::libraries::socket::register_socket_library(&self.lua);
        crate::nse::libraries::ssh2::register_ssh2_library(&self.lua);
        crate::nse::libraries::ftp::register_ftp_library(&self.lua);
        crate::nse::libraries::smtp::register_smtp_library(&self.lua);
        crate::nse::libraries::mysql::register_mysql_library(&self.lua);
        
        Ok(())
    }
    
    fn setup_require(&self, scripts_path: Arc<Mutex<Vec<PathBuf>>>) -> LuaResult<()> {
        let stdnse = self.lua.globals().get::<Table>("stdnse").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let nmap = self.lua.globals().get::<Table>("nmap").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let http = self.lua.globals().get::<Table>("http").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let comm = self.lua.globals().get::<Table>("comm").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let sslcert = self.lua.globals().get::<Table>("sslcert").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let tls = self.lua.globals().get::<Table>("tls").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let shortport = self.lua.globals().get::<Table>("shortport").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let string = self.lua.globals().get::<Table>("string").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let table_lib = self.lua.globals().get::<Table>("table").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let datetime = self.lua.globals().get::<Table>("datetime").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let socket = self.lua.globals().get::<Table>("socket").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let ssh2 = self.lua.globals().get::<Table>("ssh2").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let ftp = self.lua.globals().get::<Table>("ftp").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let smtp = self.lua.globals().get::<Table>("smtp").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        let mysql = self.lua.globals().get::<Table>("mysql").unwrap_or_else(|_| {
            self.lua.create_table().expect("Failed to create table")
        });
        
        let req_modules = self.lua.create_table().expect("Failed to create table");
        let _ = req_modules.set("stdnse", stdnse);
        let _ = req_modules.set("nmap", nmap);
        let _ = req_modules.set("http", http);
        let _ = req_modules.set("comm", comm);
        let _ = req_modules.set("sslcert", sslcert);
        let _ = req_modules.set("tls", tls);
        let _ = req_modules.set("shortport", shortport);
        let _ = req_modules.set("string", string);
        let _ = req_modules.set("table", table_lib);
        let _ = req_modules.set("datetime", datetime);
        let _ = req_modules.set("socket", socket);
        let _ = req_modules.set("ssh2", ssh2);
        let _ = req_modules.set("ftp", ftp);
        let _ = req_modules.set("smtp", smtp);
        let _ = req_modules.set("mysql", mysql);
        let _ = self.lua.globals().set("_REQUIRE_MODULES", req_modules);
        
        let scripts_path = self.scripts_path.clone();
        let require_fn = self.lua.create_function(move |lua, name: String| {
            let modules = lua.globals().get::<Table>("_REQUIRE_MODULES").expect("Failed to get modules");
            
            if let Ok(module) = modules.get::<Table>(name.as_str()) {
                return Ok(module);
            }
            
            let path_guard = scripts_path.lock().unwrap();
            for base_path in path_guard.iter() {
                let script_path = base_path.join(format!("{}.nse", name));
                if script_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&script_path) {
                        if let Ok(_loaded) = lua.load(&content).eval::<mlua::Value>() {
                            let modules = lua.globals().get::<Table>("_REQUIRE_MODULES").ok();
                            if let Some(modules) = modules {
                                if let Ok(module) = modules.get::<Table>(name.as_str()) {
                                    return Ok(module);
                                }
                            }
                        }
                    }
                }
                
                let lua_path = base_path.join(format!("{}.lua", name));
                if lua_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&lua_path) {
                        if let Ok(_loaded) = lua.load(&content).eval::<mlua::Value>() {
                            let modules = lua.globals().get::<Table>("_REQUIRE_MODULES").ok();
                            if let Some(modules) = modules {
                                if let Ok(module) = modules.get::<Table>(name.as_str()) {
                                    return Ok(module);
                                }
                            }
                        }
                    }
                }
            }
            
            Err(mlua::Error::RuntimeError(format!("module '{}' not found", name)))
        }).expect("Failed to create require function");
        
        let _ = self.lua.globals().set("require", require_fn);
        
        Ok(())
    }
    
    pub fn with_target(target: &str) -> LuaResult<Self> {
        let mut executor = Self::new()?;
        executor.set_target(target);
        Ok(executor)
    }
    
    pub fn add_scripts_path(&self, path: PathBuf) {
        if path.exists() && path.is_dir() {
            let mut paths = self.scripts_path.lock().unwrap();
            paths.push(path);
        }
    }
    
    pub fn add_default_scripts_path(&self) {
        if let Ok(home) = std::env::var("HOME") {
            let nmap_scripts = PathBuf::from(home).join(".nmap").join("nselib");
            self.add_scripts_path(nmap_scripts);
        }
        
        #[cfg(unix)]
        {
            self.add_scripts_path(PathBuf::from("/usr/share/nmap/nselib"));
            self.add_scripts_path(PathBuf::from("/usr/local/share/nmap/nselib"));
        }
        
        #[cfg(windows)]
        {
            if let Ok(program_files) = std::env::var("ProgramFiles") {
                self.add_scripts_path(PathBuf::from(program_files).join("Nmap").join("nselib"));
            }
        }
    }
    
    pub fn set_target(&mut self, target: &str) {
        self.target = target.to_string();
        
        if let Ok(nmap) = self.lua.globals().get::<mlua::Table>("nmap") {
            let _ = nmap.set("target", target);
        }
    }
    
    pub fn set_script_args(&mut self, args: &str) {
        if args.is_empty() {
            return;
        }
        
        if let Ok(stdnse) = self.lua.globals().get::<mlua::Table>("stdnse") {
            let _ = stdnse.set("script_args", args);
            
            let args_table = self.lua.create_table().ok();
            if let Some(args_table) = args_table {
                for pair in args.split(',') {
                    let pair = pair.trim();
                    if let Some((key, value)) = pair.split_once('=') {
                        let _ = args_table.set(key.trim(), value.trim());
                    }
                }
                let _ = stdnse.set("args", args_table);
            }
        }
    }
    
    pub fn run_script(&self, script: &str) -> LuaResult<String> {
        self.lua.load(script).eval::<mlua::Value>()?;
        Ok("Script executed successfully".to_string())
    }
    
    pub fn run_script_with_output(&self, script: &str) -> LuaResult<(String, Vec<String>)> {
        self.lua.load(script).eval::<mlua::Value>()?;
        
        let output = self.output.lock().unwrap().clone();
        self.output.lock().unwrap().clear();
        
        Ok(("Script executed successfully".to_string(), output))
    }
    
    pub fn run_script_async<'a>(&'a self, script: &'a str) -> impl std::future::Future<Output = LuaResult<String>> + 'a {
        async move {
            self.lua.load(script).eval::<mlua::Value>()?;
            Ok("Script executed successfully".to_string())
        }
    }
    
    pub fn run_script_file(&self, path: &std::path::Path) -> LuaResult<String> {
        let script = std::fs::read_to_string(path)?;
        self.run_script(&script)
    }
    
    pub fn run_script_file_with_output(&self, path: &std::path::Path) -> LuaResult<(String, Vec<String>)> {
        let script = std::fs::read_to_string(path)?;
        self.run_script_with_output(&script)
    }
    
    pub fn load_script(&self, name: &str) -> LuaResult<String> {
        let path_guard = self.scripts_path.lock().unwrap();
        
        for base_path in path_guard.iter() {
            let script_path = base_path.join(format!("{}.lua", name));
            if script_path.exists() {
                return Ok(std::fs::read_to_string(&script_path)?);
            }
            
            let nse_path = base_path.join(format!("{}.nse", name));
            if nse_path.exists() {
                return Ok(std::fs::read_to_string(&nse_path)?);
            }
        }
        
        Err(mlua::Error::RuntimeError(format!("Script '{}' not found in search paths", name)))
    }
    
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
    
    pub fn target(&self) -> &str {
        &self.target
    }
}

impl Default for NseExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create NSE executor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = NseExecutor::new();
        assert!(executor.is_ok());
    }
}
