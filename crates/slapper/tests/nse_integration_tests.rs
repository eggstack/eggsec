//! Integration tests for running actual NSE scripts
//!
//! These tests verify compatibility with real NSE scripts from the Nmap repository.
//! To run these tests, NSE scripts must be available in the scripts directory.

#![cfg(feature = "nse")]

#[cfg(test)]
mod integration_tests {
    use slapper::nse::executor::NseExecutor;

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
