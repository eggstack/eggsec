//! Integration tests for running actual NSE scripts
//!
//! These tests verify compatibility with real NSE scripts from the Nmap repository.
//! To run these tests, NSE scripts must be available in the scripts directory.

#![cfg(feature = "nse")]

use ipnetwork::IpNetwork;
use eggsec::nse::SandboxConfig;
use std::net::IpAddr;
use std::path::PathBuf;
use tempfile::TempDir;

#[cfg(test)]
mod sandbox_enforcement_tests {
    use super::*;

    fn create_sandbox_with_allowed_dir(path: &str) -> SandboxConfig {
        SandboxConfig {
            enabled: true,
            allowed_dir: Some(PathBuf::from(path)),
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks: Vec::new(),
        }
    }

    fn create_sandbox_with_commands(commands: Vec<&str>) -> SandboxConfig {
        SandboxConfig {
            enabled: true,
            allowed_dir: Some(PathBuf::from("/tmp")),
            allowed_commands: commands.into_iter().map(String::from).collect(),
            log_violations: true,
            allowed_networks: Vec::new(),
        }
    }

    fn create_sandbox_with_networks(networks: Vec<&str>) -> SandboxConfig {
        let allowed_networks: Vec<IpNetwork> = networks
            .into_iter()
            .filter_map(|n| n.parse().ok())
            .collect();
        SandboxConfig {
            enabled: true,
            allowed_dir: Some(PathBuf::from("/tmp")),
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks,
        }
    }

    #[test]
    fn test_path_sandbox_blocks_parent_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_str().unwrap();
        let sandbox = create_sandbox_with_allowed_dir(allowed_path);

        let blocked_path = format!("{}/../../../etc/passwd", allowed_path);
        assert!(
            sandbox.get_allowed_path(&blocked_path).is_none(),
            "Path traversal with '..' should be blocked"
        );
    }

    #[test]
    fn test_path_sandbox_allows_inside_allowed_dir() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_str().unwrap();
        let sandbox = create_sandbox_with_allowed_dir(allowed_path);

        let file_path = temp_dir.path().join("test_file.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = sandbox.get_allowed_path(file_path.to_str().unwrap());
        assert!(
            result.is_some(),
            "Files inside allowed directory should be accessible"
        );
    }

    #[test]
    fn test_path_sandbox_blocks_outside_allowed_dir() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_str().unwrap();
        let sandbox = create_sandbox_with_allowed_dir(allowed_path);

        let result = sandbox.get_allowed_path("/etc/passwd");
        assert!(
            result.is_none(),
            "Paths outside allowed directory should be blocked"
        );
    }

    #[test]
    fn test_path_sandbox_with_symlink_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_str().unwrap();
        let sandbox = create_sandbox_with_allowed_dir(allowed_path);

        let link_path = temp_dir.path().join("link_to_etc");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", &link_path).unwrap();

        let through_symlink = format!("{}/link_to_etc/passwd", allowed_path);
        let result = sandbox.get_allowed_path(&through_symlink);
        assert!(
            result.is_none(),
            "Accessing files through symlinks outside allowed dir should be blocked"
        );
    }

    #[test]
    fn test_path_sandbox_resolves_symlinks_inside_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_str().unwrap();
        let sandbox = create_sandbox_with_allowed_dir(allowed_path);

        let sub_dir = temp_dir.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();

        let file_path = sub_dir.join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        let link_path = temp_dir.path().join("link_to_file");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&file_path, &link_path).unwrap();

        let link_result = sandbox.get_allowed_path(link_path.to_str().unwrap());
        assert!(
            link_result.is_some(),
            "Symlinks to files inside allowed dir should work"
        );
    }

    #[test]
    fn test_command_sandbox_blocks_all_when_empty() {
        let sandbox = create_sandbox_with_commands(vec![]);

        assert!(
            !sandbox.is_command_allowed("ls"),
            "No commands should be allowed when allowed_commands is empty"
        );
        assert!(
            !sandbox.is_command_allowed("cat"),
            "cat should not be allowed when allowed_commands is empty"
        );
        assert!(
            !sandbox.is_command_allowed("curl"),
            "curl should not be allowed when allowed_commands is empty"
        );
    }

    #[test]
    fn test_command_sandbox_allows_specific_commands() {
        let sandbox = create_sandbox_with_commands(vec!["ls", "cat"]);

        assert!(sandbox.is_command_allowed("ls"), "ls should be allowed");
        assert!(sandbox.is_command_allowed("cat"), "cat should be allowed");
        assert!(
            sandbox.is_command_allowed("cat /etc/passwd"),
            "cat with args should be allowed if cat is in list"
        );
    }

    #[test]
    fn test_command_sandbox_blocks_unlisted_commands() {
        let sandbox = create_sandbox_with_commands(vec!["ls", "cat"]);

        assert!(
            !sandbox.is_command_allowed("rm"),
            "rm should not be allowed"
        );
        assert!(
            !sandbox.is_command_allowed("curl"),
            "curl should not be allowed"
        );
        assert!(
            !sandbox.is_command_allowed("wget"),
            "wget should not be allowed"
        );
    }

    #[test]
    fn test_command_sandbox_extracts_command_name() {
        let sandbox = create_sandbox_with_commands(vec!["ls"]);

        assert!(
            sandbox.is_command_allowed("ls -la"),
            "ls -la should match 'ls' command"
        );
        assert!(
            sandbox.is_command_allowed("ls  -la"),
            "ls with multiple spaces should match"
        );
    }

    #[test]
    fn test_network_sandbox_allows_matching_ip() {
        let sandbox = create_sandbox_with_networks(vec!["192.168.1.0/24", "10.0.0.0/8"]);

        assert!(
            sandbox.is_network_allowed(IpAddr::from([192, 168, 1, 100])),
            "IP in 192.168.1.0/24 should be allowed"
        );
        assert!(
            sandbox.is_network_allowed(IpAddr::from([10, 20, 30, 40])),
            "IP in 10.0.0.0/8 should be allowed"
        );
    }

    #[test]
    fn test_network_sandbox_blocks_non_matching_ip() {
        let sandbox = create_sandbox_with_networks(vec!["192.168.1.0/24"]);

        assert!(
            !sandbox.is_network_allowed(IpAddr::from([192, 168, 2, 1])),
            "IP outside allowed network should be blocked"
        );
        assert!(
            !sandbox.is_network_allowed(IpAddr::from([8, 8, 8, 8])),
            "Public IP outside allowed network should be blocked"
        );
    }

    #[test]
    fn test_network_sandbox_allows_all_when_empty() {
        let sandbox = SandboxConfig {
            enabled: true,
            allowed_dir: Some(PathBuf::from("/tmp")),
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks: Vec::new(),
        };

        assert!(
            sandbox.is_network_allowed(IpAddr::from([8, 8, 8, 8])),
            "All IPs should be allowed when allowed_networks is empty"
        );
        assert!(
            sandbox.is_network_allowed(IpAddr::from([1, 1, 1, 1])),
            "Any IP should work when no restrictions"
        );
    }

    #[test]
    fn test_network_sandbox_disabled_allows_all() {
        let sandbox = SandboxConfig {
            enabled: false,
            allowed_dir: Some(PathBuf::from("/tmp")),
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks: vec!["192.168.1.0/24".parse().unwrap()],
        };

        assert!(
            sandbox.is_network_allowed(IpAddr::from([8, 8, 8, 8])),
            "When sandbox is disabled, all IPs should be allowed"
        );
    }

    #[test]
    fn test_host_resolution_with_allowlist() {
        let sandbox = create_sandbox_with_networks(vec!["127.0.0.0/8"]);

        let localhost_ips = sandbox.resolve_host("localhost");
        assert!(
            !localhost_ips.is_empty(),
            "localhost should resolve to at least one IP"
        );
        assert!(
            localhost_ips
                .iter()
                .all(|ip| sandbox.is_network_allowed(*ip)),
            "All resolved IPs should be in allowed networks"
        );
    }

    #[test]
    fn test_sandbox_config_enabled_default() {
        let sandbox = SandboxConfig::enabled();
        assert!(sandbox.enabled, "Sandbox should be enabled");
        assert!(
            sandbox.allowed_dir.is_some(),
            "Default allowed_dir should be set"
        );
    }

    #[test]
    fn test_sandbox_config_disabled_allows_all() {
        let sandbox = SandboxConfig {
            enabled: false,
            allowed_dir: None,
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks: Vec::new(),
        };

        assert!(
            sandbox.get_allowed_path("/any/path").is_some(),
            "All paths should be allowed when disabled"
        );
        assert!(
            sandbox.is_command_allowed("any_command"),
            "All commands should be allowed when disabled"
        );
        assert!(
            sandbox.is_network_allowed(IpAddr::from([1, 2, 3, 4])),
            "All IPs should be allowed when disabled"
        );
    }

    #[test]
    fn test_path_sandbox_with_none_allowed_dir() {
        let sandbox = SandboxConfig {
            enabled: true,
            allowed_dir: None,
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks: Vec::new(),
        };

        let result = sandbox.get_allowed_path("/etc/passwd");
        assert!(
            result.is_some(),
            "When allowed_dir is None, all paths should be allowed"
        );
    }

    #[test]
    fn test_ipv6_network_sandbox() {
        let sandbox = create_sandbox_with_networks(vec!["::1/128", "fc00::/7"]);

        assert!(
            sandbox.is_network_allowed(IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1])),
            "IPv6 localhost should be allowed"
        );
        assert!(
            !sandbox.is_network_allowed(IpAddr::from([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 1])),
            "IPv6 outside allowed networks should be blocked"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use eggsec::nse::executor::NseExecutor;

    /// Test running a simple NSE script that uses http library
    #[test]
    fn test_nse_script_http_enum() {
        let mut executor = NseExecutor::new().unwrap();

        // Set target
        executor.set_target("example.com").unwrap();

        // Common HTTP enumeration script pattern
        let script = r#"
            local http = require "http"
            local shortport = require "shortport"
            
            portrule = shortport.http
            
            action = function(host, port)
                local response = http.get(host.ip, port.number, "/")
                if response.status == 200 then
                    return "Found web server"
                end
                return nil
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok(), "Script should execute without error");
    }

    /// Test running a script that uses stdnse library functions
    #[test]
    fn test_nse_script_stdns_usage() {
        let executor = NseExecutor::new().unwrap();

        let script = r#"
            local stdnse = require "stdnse"
            local shortport = require "shortport"
            
            portrule = shortport.portnumber(80)
            
            action = function(host, port)
                -- Test stdnse functions
                stdnse.debug1("Debug message")
                stdnse.verbose1("Verbose message")
                
                -- Test output table
                local output = stdnse.output_table()
                output.status = "open"
                output.service = "http"
                
                return output
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test running a script that uses regex (re library)
    #[test]
    fn test_nse_script_regex_usage() {
        let executor = NseExecutor::new().unwrap();

        let script = r#"
            local re = require "re"
            
            -- Test basic regex matching
            local result = re.match("hello world", "hello")
            if result then
                return "Pattern matched"
            end
            return "No match"
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Pattern matched"));
    }

    /// Test running a script that uses httpspider
    #[test]
    fn test_nse_script_httpspider_usage() {
        let executor = NseExecutor::new().unwrap();

        let script = r#"
            local httpspider = require "httpspider"
            
            -- Test httpspider.parse function
            local html = [[
                <html>
                <body>
                    <a href="/page1">Page 1</a>
                    <a href="/page2">Page 2</a>
                    <img src="/image.jpg"/>
                </body>
                </html>
            ]]
            
            local parsed = httpspider.parse(html, "http://example.com/")
            return "Links found: " .. #parsed.links
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test running a script with multiple library dependencies
    #[test]
    fn test_nse_script_multi_lib() {
        let executor = NseExecutor::new().unwrap();

        let script = r#"
            local http = require "http"
            local stdnse = require "stdnse"
            local shortport = require "shortport"
            local re = require "re"
            local json = require "json"
            
            portrule = shortport.http
            
            action = function(host, port)
                -- Test multiple libraries working together
                local response = http.get(host.ip, port.number, "/")
                
                if response.status == 200 then
                    -- Use regex to find patterns in body
                    local title = re.match(response.body, "<title>(.-)</title>")
                    
                    -- Use JSON to create structured output
                    local output = {
                        status = response.status,
                        title = title and title[1] or "unknown",
                        urls = {}
                    }
                    
                    return stdnse.output_table(output)
                end
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test script with script arguments
    #[test]
    fn test_nse_script_with_args() {
        let mut executor = NseExecutor::new().unwrap();

        // Set script arguments
        executor.set_script_args("http.title=Test").unwrap();

        let script = r#"
            local stdnse = require "stdnse"
            
            action = function(host, port)
                local title = stdnse.get_script_args("http.title")
                return "Title arg: " .. tostring(title)
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test running a script that uses datafiles
    #[test]
    fn test_nse_script_datafiles() {
        let executor = NseExecutor::new().unwrap();

        let script = r#"
            local datafiles = require "datafiles"
            
            action = function(host, port)
                -- Try to parse nmap service files
                local services = datafiles.parse_services()
                if services then
                    return "Services loaded: " .. #services
                end
                return "Services not available"
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test running a script that uses target library
    #[test]
    fn test_nse_script_target() {
        let mut executor = NseExecutor::new().unwrap();
        executor.set_target("192.168.1.1").unwrap();

        let script = r#"
            local target = require "target"
            local nmap = require "nmap"
            
            action = function(host, port)
                -- Test target library
                local hostname = target.hostname()
                local ip = target.ip()
                
                -- Test nmap library
                local port_state = nmap.get_port_state(host, port)
                
                return "Hostname: " .. tostring(hostname) .. ", IP: " .. tostring(ip)
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test running a brute force script pattern
    #[test]
    fn test_nse_script_brute_pattern() {
        let executor = NseExecutor::new().unwrap();

        // Simple test that brute library can be required
        let script = r#"
            local brute = require "brute"
            local creds = require "creds"
            
            -- Just verify libraries can be loaded
            return "Brute libraries available"
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }

    /// Test socket library usage
    #[test]
    fn test_nse_script_socket() {
        let executor = NseExecutor::new().unwrap();

        let script = r#"
            local socket = require "socket"
            
            action = function(host, port)
                -- Test TCP connection
                local status, err = socket.connect("127.0.0.1", 80)
                
                -- Even if connection fails, the library is available
                return "Socket library available"
            end
        "#;

        let result = executor.run_script(script);
        assert!(result.is_ok());
    }
}
