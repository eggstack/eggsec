//! Integration tests for running actual NSE scripts
//!
//! This test suite validates compatibility with real NSE scripts from the Nmap repository.

#![cfg(feature = "nse")]

use eggsec::nse::executor::NseExecutor;

#[test]
fn test_http_enum_script() {
    let mut executor = NseExecutor::new().unwrap();
    executor.set_target("example.com").unwrap();

    let script = r#"
        local http = require "http"
        local shortport = require "shortport"
        
        portrule = shortport.http
        
        action = function(host, port)
            local response = http.get(host.ip, port.number, "/")
            return "status:" .. response.status
        end
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok(), "Script should execute: {:?}", result);
}

#[test]
fn test_http_title_script() {
    let mut executor = NseExecutor::new().unwrap();
    executor.set_target("example.com").unwrap();

    let script = r#"
        local http = require "http"
        local shortport = require "shortport"
        local re = require "re"
        
        portrule = shortport.http
        
        action = function(host, port)
            local response = http.get(host.ip, port.number, "/")
            if response.status == 200 then
                local title = re.match(response.body, "<title>(.-)</title>")
                if title then
                    return title[1]
                end
            end
            return nil
        end
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_http_pipeline_script() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local httppipeline = require "httppipeline"
        
        -- Test creating a pipeline
        local pipeline = httppipeline.new("example.com", 80, {timeout = 10})
        
        -- Add requests
        httppipeline.add(pipeline, "GET", "/")
        httppipeline.add(pipeline, "GET", "/about")
        
        -- Execute pipeline
        local responses = httppipeline.go(pipeline)
        
        return #responses .. " responses"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok(), "Pipeline should work: {:?}", result);
}

#[test]
fn test_httpspider_crawl_script() {
    let executor = NseExecutor::new().unwrap();

    // Test basic httpspider functionality
    let script = r#"
        local httpspider = require "httpspider"
        
        -- Test parse function works
        local html = '<html><body><a href="/test">Test</a></body></html>'
        local parsed = httpspider.parse(html, "http://example.com/")
        
        return #parsed.links .. " links found"
    "#;

    let result = executor.run_script(script);
    let is_ok = result.is_ok();
    match result {
        Ok(r) => println!("Result: {:?}", r),
        Err(e) => println!("Error: {:?}", e),
    }
    assert!(is_ok);
}

#[test]
fn test_stdnse_debug_script() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local stdnse = require "stdnse"
        
        return "ok"
    "#;

    let result = executor.run_script(script);
    let is_ok = result.is_ok();
    match result {
        Ok(r) => println!("Result: {:?}", r),
        Err(e) => println!("Error: {:?}", e),
    }
    assert!(is_ok);
}

#[test]
fn test_json_encode_decode() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local json = require "json"
        
        local data = {name = "test", value = 123}
        local encoded = json.encode(data)
        local decoded = json.decode(encoded)
        
        return decoded.name
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("test"));
}

#[test]
fn test_regex_matching() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local re = require "re"
        
        local result = re.match("hello world", "hello")
        if result then
            return "matched"
        end
        return "no match"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("matched"));
}

#[test]
fn test_url_parsing() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local url = require "url"
        
        local parsed = url.parse("http://example.com/path?query=value")
        return parsed.host
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("example.com"));
}

#[test]
fn test_base64_encoding() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local base64 = require "base64"
        
        local encoded = base64.encode("hello")
        return encoded
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_smb_basic_script() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local smb = require "smb"
        
        -- Just test library loads
        return "smb loaded"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_ssh_basic_script() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local ssh = require "ssh"
        
        -- Just test library loads
        return "ssh loaded"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_ldap_basic_script() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local ldap = require "ldap"
        
        -- Just test library loads
        return "ldap loaded"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_snmp_basic_script() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local snmp = require "snmp"
        
        -- Just test library loads
        return "snmp loaded"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_vulns_lookup() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local vulns = require "vulns"
        
        local known = vulns.is_known("CVE-2017-0144")
        return tostring(known)
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("true"));
}

#[test]
fn test_tableaux_functions() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local tableaux = require "tableaux"
        
        local t = {a = 1, b = 2}
        local keys = tableaux.keys(t)
        
        return #keys .. " keys"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_outlib_functions() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local outlib = require "outlib"
        
        local t = {"a", "b", "c"}
        local list = outlib.list_sep(t)
        
        return list
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_datafiles_parsing() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local datafiles = require "datafiles"
        
        -- Test library functions exist
        return type(datafiles.parse_services)
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_nmap_registry() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local nmap = require "nmap"
        
        return "nmap loaded"
    "#;

    let result = executor.run_script(script);
    let is_ok = result.is_ok();
    match result {
        Ok(r) => println!("Result: {:?}", r),
        Err(e) => println!("Error: {:?}", e),
    }
    assert!(is_ok);
}

#[test]
fn test_target_library() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local target = require "target"
        
        return "target loaded"
    "#;

    let result = executor.run_script(script);
    let is_ok = result.is_ok();
    match result {
        Ok(r) => println!("Result: {:?}", r),
        Err(e) => println!("Error: {:?}", e),
    }
    assert!(is_ok);
}

#[test]
fn test_shortport_http() {
    let mut executor = NseExecutor::new().unwrap();
    executor.set_target("example.com").unwrap();

    let script = r#"
        local shortport = require "shortport"
        
        local port = {
            number = 80,
            protocol = "tcp",
            service = {name = "http"}
        }
        
        local is_http = shortport.http(port)
        
        return tostring(is_http)
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}

#[test]
fn test_creds_library() {
    let executor = NseExecutor::new().unwrap();

    let script = r#"
        local creds = require "creds"
        
        return "creds loaded"
    "#;

    let result = executor.run_script(script);
    assert!(result.is_ok());
}
