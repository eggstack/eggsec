//! NSE (Nmap Scripting Engine) support for Slapper
//!
//! This module provides the ability to run Nmap NSE scripts using a Lua interpreter.
//! It leverages mlua (Lua 5.4) and wraps existing Slapper functionality
//! to provide NSE-compatible libraries.

use crate::cli::NseArgs;
use crate::config::SlapperConfig;

pub mod executor;
pub mod libraries;
pub mod tool;

pub use executor::NseExecutor;
pub use tool::NseTool;

pub async fn run_cli(args: NseArgs, _config: &SlapperConfig) -> anyhow::Result<()> {
    let target = &args.target;
    let script = &args.script;
    let script_args = args.script_args.unwrap_or_default();
    
    println!("Running NSE script '{}' against '{}'", script, target);
    
    let mut executor = NseExecutor::with_target(target)
        .map_err(|e| anyhow::anyhow!("Failed to create NSE executor: {}", e))?;
    executor.set_script_args(&script_args);
    
    let script_content = if let Some(ref script_file) = args.script_file {
        std::fs::read_to_string(script_file)?
    } else {
        get_builtin_script(script)
    };
    
    let result = executor.run_script(&script_content)
        .map_err(|e| anyhow::anyhow!("Script execution failed: {}", e))?;
    
    if args.json {
        let output = serde_json::json!({
            "target": target,
            "script": script,
            "script_args": script_args,
            "output": result,
            "success": true
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Target: {}", target);
        println!("Script: {}", script);
        println!("Result: {}", result);
    }
    
    Ok(())
}

fn get_builtin_script(name: &str) -> String {
    match name {
        "default" | "discovery" => {
            r#"
-- Default NSE discovery script
local stdnse = require "stdnse"

stdnse.verbose = 1

local host = nmap.target
if host and host ~= "" then
    stdnse.format_output({status = "open", service = "discovered"}, {separator = ", "})
end

return "NSE scan complete"
"#.to_string()
        }
        "banner" => {
            r#"
-- Banner grabbing script
local comm = require "comm"
local stdnse = require "stdnse"

local host = nmap.target
local port = 80

return "Banner grab ready - use external script for full functionality"
"#.to_string()
        }
        "http-headers" => {
            r#"
-- HTTP headers discovery script
local http = require "http"
local stdnse = require "stdnse"

local host = nmap.target

return "HTTP headers scan ready - use external script for full functionality"
"#.to_string()
        }
        _ => {
            format!(
                r#"
-- Custom NSE script: {}
local stdnse = require "stdnse"

stdnse.verbose = 1

return "Custom script '{}' executed - full NSE library support coming soon"
"#,
                name, name
            )
        }
    }
}
