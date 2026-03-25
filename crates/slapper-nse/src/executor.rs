//! NSE Executor - Synchronous Lua VM wrapper with rule execution
//!
//! Wraps ExecutorCore and adds NSE rule evaluation (prerule, hostrule,
//! portrule, postrule) and category management.

use mlua::{Lua, Result as LuaResult, Table, Value};
use std::path::PathBuf;

use crate::executor_core::ExecutorCore;

pub struct NseExecutor {
    core: ExecutorCore,
}

impl NseExecutor {
    pub fn new() -> LuaResult<Self> {
        Ok(Self {
            core: ExecutorCore::new()?,
        })
    }

    pub fn with_target(target: &str) -> LuaResult<Self> {
        let mut exec = Self::new()?;
        exec.set_target(target);
        Ok(exec)
    }

    // Delegate core accessors
    pub fn lua(&self) -> &Lua {
        self.core.lua()
    }
    pub fn target(&self) -> &str {
        self.core.target()
    }
    pub fn set_target(&mut self, target: &str) -> Result<(), String> {
        self.core.set_target(target)
    }
    pub fn add_scripts_path(&self, path: PathBuf) {
        self.core.add_scripts_path(path);
    }
    pub fn add_default_scripts_path(&self) {
        self.core.add_default_scripts_path();
    }
    pub fn set_script_args(&mut self, args: &str) -> Result<(), String> {
        self.core.set_script_args(args)
    }
    pub fn add_output(&self, output: String) -> Result<(), String> {
        self.core.add_output(output)
    }
    pub fn get_output(&self) -> Result<Vec<String>, String> {
        Ok(self.core.get_output())
    }
    pub fn get_script_output(&self) -> Result<String, String> {
        self.core.get_script_output()
    }
    pub fn run_script(&self, script: &str) -> LuaResult<String> {
        self.core.run_script(script)
    }
    pub fn load_script(&self, name: &str) -> LuaResult<String> {
        self.core.load_script(name)
    }
    pub fn set_host_info(
        &mut self,
        hostname: Option<String>,
        ip: String,
        mac: Option<String>,
        status: Option<String>,
    ) -> Result<(), String> {
        self.core.set_host_info(hostname, ip, mac, status)
    }
    pub fn add_port(
        &mut self,
        port: u16,
        protocol: &str,
        state: &str,
        service: Option<String>,
    ) -> Result<(), String> {
        self.core.add_port(port, protocol, state, service)
    }

    // Executor-specific: rule execution

    pub fn run_script_with_rules(&mut self, script: &str) -> LuaResult<(String, Vec<String>)> {
        self.lua().load(script).eval::<Value>()?;
        let globals = self.lua().globals();
        let mut outputs = Vec::new();

        // prerule
        if let Ok(prerule) = globals.get::<mlua::Function>("prerule") {
            match prerule.call::<Value>(()) {
                Ok(r) if !r.is_nil() => outputs.push(format!("prerule: {:?}", r)),
                Err(e) => outputs.push(format!("prerule error: {}", e)),
                _ => {}
            }
        }

        // hostrule
        let hostrule_matched = if let Ok(hostrule) = globals.get::<mlua::Function>("hostrule") {
            let host = globals.get::<Table>("nmap")?;
            match hostrule.call::<Value>(host.clone()) {
                Ok(r) if r.as_boolean().unwrap_or(false) => {
                    if let Ok(action) = globals.get::<mlua::Function>("action") {
                        match action.call::<Value>((host.clone(), self.lua().create_table()?)) {
                            Ok(v) if !v.is_nil() => outputs.push(format!("action: {:?}", v)),
                            Err(e) => outputs.push(format!("action error: {}", e)),
                            _ => {}
                        }
                    }
                    true
                }
                Err(e) => {
                    outputs.push(format!("hostrule error: {}", e));
                    false
                }
                _ => false,
            }
        } else {
            false
        };

        // portrule
        let ports = globals.get::<Table>("nmap")?.get::<Table>("_ports")?;
        let mut portrule_matched = false;

        for pair in ports.pairs::<String, Table>() {
            if let Ok((_, port_info)) = pair {
                if let Ok(portrule) = globals.get::<mlua::Function>("portrule") {
                    match portrule.call::<Value>(port_info.clone()) {
                        Ok(r) if r.as_boolean().unwrap_or(false) => {
                            if let Ok(action) = globals.get::<mlua::Function>("action") {
                                let host = globals.get::<Table>("nmap")?;
                                match action.call::<Value>((host.clone(), port_info.clone())) {
                                    Ok(v) if !v.is_nil() => {
                                        outputs.push(format!("action: {:?}", v))
                                    }
                                    Err(e) => outputs.push(format!("action error: {}", e)),
                                    _ => {}
                                }
                            }
                            portrule_matched = true;
                            break;
                        }
                        Err(e) => outputs.push(format!("portrule error: {}", e)),
                        _ => {}
                    }
                }
            }
        }

        // postrule
        if let Ok(postrule) = globals.get::<mlua::Function>("postrule") {
            match postrule.call::<Value>(()) {
                Ok(r) if !r.is_nil() => outputs.push(format!("postrule: {:?}", r)),
                Err(e) => outputs.push(format!("postrule error: {}", e)),
                _ => {}
            }
        }

        if let Ok(script_output) = self.get_script_output() {
            if !script_output.is_empty() {
                outputs.push(script_output);
            }
        }

        if outputs.is_empty() && !hostrule_matched && !portrule_matched {
            outputs.push("No rules matched or no output generated".to_string());
        }

        Ok((outputs.join("\n"), outputs))
    }

    pub fn check_portrule(
        &mut self,
        portrule: Option<&str>,
        port: u16,
        protocol: &str,
        state: &str,
        service: Option<&str>,
    ) -> LuaResult<bool> {
        let globals = self.lua().globals();
        let port_table = self.lua().create_table()?;
        port_table.set("number", port)?;
        port_table.set("protocol", protocol)?;
        port_table.set("state", state)?;
        if let Some(svc) = service {
            port_table.set("service", svc)?;
        }

        if let Some(rule) = portrule {
            if !rule.is_empty() {
                if let Ok(f) = self.lua().load(rule).eval::<mlua::Function>() {
                    if let Ok(r) = f.call::<Value>(port_table.clone()) {
                        return Ok(r.as_boolean().unwrap_or(false));
                    }
                }
            }
        }

        if let Ok(f) = globals.get::<mlua::Function>("portrule") {
            if let Ok(r) = f.call::<Value>(port_table) {
                return Ok(r.as_boolean().unwrap_or(false));
            }
        }
        Ok(true)
    }

    pub fn check_hostrule(&mut self, hostrule: Option<&str>) -> LuaResult<bool> {
        let globals = self.lua().globals();
        let host = match globals.get::<Table>("nmap") {
            Ok(t) => t,
            Err(_) => return Ok(true),
        };

        if let Some(rule) = hostrule {
            if !rule.is_empty() {
                if let Ok(f) = self.lua().load(rule).eval::<mlua::Function>() {
                    if let Ok(r) = f.call::<Value>(host.clone()) {
                        return Ok(r.as_boolean().unwrap_or(false));
                    }
                }
            }
        }

        if let Ok(f) = globals.get::<mlua::Function>("hostrule") {
            if let Ok(r) = f.call::<Value>(host) {
                return Ok(r.as_boolean().unwrap_or(false));
            }
        }
        Ok(true)
    }

    pub fn get_prerule_result(&self) -> Option<String> {
        let f = self.lua().globals().get::<mlua::Function>("prerule").ok()?;
        let r = f.call::<Value>(()).ok()?;
        Some(format!("{:?}", r))
    }

    pub fn get_postrule_result(&self) -> Option<String> {
        let f = self
            .lua()
            .globals()
            .get::<mlua::Function>("postrule")
            .ok()?;
        let r = f.call::<Value>(()).ok()?;
        Some(format!("{:?}", r))
    }

    pub fn run_script_with_output(&self, script: &str) -> LuaResult<(String, Vec<String>)> {
        self.lua().load(script).eval::<Value>()?;
        let output = self.core.get_output();
        Ok(("Script executed successfully".to_string(), output))
    }

    pub fn run_script_file(&self, path: &std::path::Path) -> LuaResult<String> {
        let script = std::fs::read_to_string(path)?;
        self.run_script(&script)
    }

    pub fn run_script_file_with_output(
        &self,
        path: &std::path::Path,
    ) -> LuaResult<(String, Vec<String>)> {
        let script = std::fs::read_to_string(path)?;
        self.run_script_with_output(&script)
    }

    pub fn check_script_category(&self, script_name: &str, category: &str) -> bool {
        let categories = get_script_categories();
        if let Some(cats) = categories.get(script_name) {
            return cats.contains(&category);
        }
        matches!(category, "default" | "safe")
    }

    pub fn get_script_categories(&self, script_name: &str) -> Vec<String> {
        let categories = get_script_categories();
        categories
            .get(script_name)
            .map(|cats| cats.iter().map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["default".to_string()])
    }

    pub fn get_category_scripts(&self, category: &str) -> Vec<String> {
        get_script_categories()
            .into_iter()
            .filter(|(_, cats)| cats.contains(&category))
            .map(|(name, _)| name.to_string())
            .collect()
    }
}

fn get_script_categories() -> std::collections::HashMap<&'static str, Vec<&'static str>> {
    let mut m = std::collections::HashMap::new();
    m.insert("http-title", vec!["default", "discovery", "safe"]);
    m.insert("http-headers", vec!["default", "discovery", "safe"]);
    m.insert("http-methods", vec!["default", "discovery", "safe"]);
    m.insert("http-robots.txt", vec!["discovery", "safe"]);
    m.insert("ssh2-enum-algos", vec!["discovery", "safe"]);
    m.insert("banner", vec!["default", "discovery"]);
    m.insert("broadcast", vec!["broadcast"]);
    m.insert("smb-brute", vec!["brute", "intrusive"]);
    m.insert("http-brute", vec!["brute", "intrusive"]);
    m.insert("ftp-brute", vec!["brute", "intrusive"]);
    m.insert("ssh-brute", vec!["brute", "intrusive"]);
    m.insert("mysql-enum", vec!["auth", "default"]);
    m.insert("smb-enum-users", vec!["discovery", "safe"]);
    m.insert("smb-enum-shares", vec!["discovery", "safe"]);
    m.insert("vuln", vec!["vuln", "safe"]);
    m.insert("exploit", vec!["exploit", "intrusive"]);
    m.insert("dos", vec!["dos", "intrusive"]);
    m
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
