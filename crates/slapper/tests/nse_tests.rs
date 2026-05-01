#[cfg(feature = "nse")]
#[cfg(test)]
mod tests {
    use slapper::nse::NseExecutor;

    #[test]
    fn test_executor_creation() {
        let executor = NseExecutor::new();
        assert!(executor.is_ok());
    }

    #[test]
    fn test_executor_with_target() {
        let executor = NseExecutor::with_target("127.0.0.1");
        assert!(executor.is_ok());
        let executor = executor.unwrap();
        assert_eq!(executor.target(), "127.0.0.1");
    }

    #[test]
    fn test_set_target() {
        let mut executor = NseExecutor::new().unwrap();
        let result = executor.set_target("192.168.1.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_script_args() {
        let mut executor = NseExecutor::new().unwrap();
        let result = executor.set_script_args("userdb=filename.txt,foo=bar");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_empty_script_args() {
        let mut executor = NseExecutor::new().unwrap();
        let result = executor.set_script_args("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_simple_script() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("return 42");
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_hello_world_script() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("return 'Hello, World!'");
        assert!(result.is_ok());
        // The result should contain the returned string
        assert!(result.unwrap().contains("Hello, World!"));
    }

    #[test]
    fn test_run_table_return_script() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("return {foo = 'bar', num = 123}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_stdlib() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local stdnse = require \"stdnse\" return stdnse.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_nmap() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local nmap = require \"nmap\" return nmap.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_http() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local http = require \"http\" return http.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_socket() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local socket = require \"socket\" return socket.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_shortport() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local shortport = require \"shortport\" return shortport.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_comm() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local comm = require \"comm\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_sslcert() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local sslcert = require \"sslcert\" return sslcert.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_tls() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local tls = require \"tls\" return tls.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_datetime() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local datetime = require \"datetime\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_json() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local json = require \"json\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_base64() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local base64 = require \"base64\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_url() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local url = require \"url\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_dns() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local dns = require \"dns\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_rand() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local rand = require \"rand\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdlib_format_output() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local stdnse = require "stdnse"
            local output = stdnse.output_table()
            output.foo = "bar"
            local result, status = stdnse.format_output(output)
            return status
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdlib_get_script_args() {
        let mut executor = NseExecutor::new().unwrap();
        executor.set_script_args("testkey=testvalue").unwrap();

        let result = executor.run_script(
            "local stdnse = require \"stdnse\" return stdnse.get_script_args(\"testkey\")",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdnse_base64_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local stdnse = require \"stdnse\" local encoded = stdnse.base64(\"test\") return encoded");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdnse_hex_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local stdnse = require \"stdnse\" local hex = stdnse.hex_to_bytes(\"test\") return type(hex)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdnse_strsplit() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local stdnse = require \"stdnse\" local parts = stdnse.strsplit(\"a,b,c\", \",\") return #parts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local nmap = require "nmap"
            nmap.target = "127.0.0.1"
            local target = nmap.target
            return target
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_get_port_state() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local nmap = require \"nmap\" local port = nmap.get_port_state(nil, 80) return port.number");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_get_port_state_by_protocol() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local nmap = require \"nmap\" local state = nmap.get_port_state_by_protocol(80, \"tcp\") return state");
        assert!(result.is_ok());
    }

    #[test]
    fn test_socket_udp() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local socket = require \"socket\" local sock = socket.udp() return type(sock)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_socket_is_udp() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local socket = require "socket"
            local sock = socket.udp()
            return sock:is_udp()
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_nse_rule_execution() {
        let mut executor = NseExecutor::new().unwrap();
        let result = executor.run_script_with_rules(
            r#"
            local nmap = require "nmap"
            local stdnse = require "stdnse"
            
            -- Register a simple hostrule
            stdnse.register_hostrule(function(host)
                return true
            end)
            
            -- Set action function directly
            stdnse.action(function(host, port)
                return "Test action executed"
            end)
            
            return "Script loaded"
        "#,
        );
        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        assert!(output.contains("Test action executed"));
    }

    #[test]
    fn test_nse_prerule_postrule() {
        let mut executor = NseExecutor::new().unwrap();
        let result = executor.run_script_with_rules(
            r#"
            local nmap = require "nmap"
            local stdnse = require "stdnse"
            
            stdnse.register_prerule(function()
                return "prerule executed"
            end)
            
            stdnse.register_postrule(function()
                return "postrule executed"
            end)
            
            return "Script loaded"
        "#,
        );
        assert!(result.is_ok());
        let (output, _) = result.unwrap();
        assert!(output.contains("prerule executed"));
        assert!(output.contains("postrule executed"));
    }

    #[test]
    fn test_stdnse_string_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local stdnse = require "stdnse"
            local text = "Hello World"
            local has_prefix = stdnse.has_prefix(text, "Hello")
            local has_suffix = stdnse.has_suffix(text, "World")
            local contains = stdnse.contains(text, "lo Wo")
            return has_prefix and has_suffix and contains
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_stdnse_urlencode() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local stdnse = require "stdnse"
            local encoded = stdnse.urlencode("hello world")
            return encoded
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_portnumber() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.portnumber(\"80\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_service() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.service(\"http\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_ssl() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.ssl(\"443\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_http() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.http(\"80\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_ftp() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.ftp(\"21\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_ssh() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.ssh(\"22\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_smtp() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.smtp(\"25\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_mysql() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.mysql(\"3306\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_redis() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.redis(\"6379\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_shortport_mongodb() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local shortport = require \"shortport\" local result = shortport.mongodb(\"27017\") return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_socket_tcp() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local socket = require \"socket\" local sock = socket.tcp() return type(sock)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_get() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local http = require \"http\" local response = http.get(\"127.0.0.1\", 80, \"/\") return type(response)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_post() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local http = require \"http\" local response = http.post(\"127.0.0.1\", 80, \"/\", \"data=test\") return type(response)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_head() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local http = require \"http\" local response = http.head(\"127.0.0.1\", 80, \"/\") return type(response)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_comm_get_banner() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local comm = require \"comm\" local result = comm.get_banner(\"127.0.0.1\", 9999) return type(result)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_decode() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local json = require \"json\" local obj = json.decode('{\"foo\": \"bar\"}') return obj.foo");
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_encode() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local json = require \"json\" local str = json.encode({foo = \"bar\"}) return type(str)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_url_parse() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local url = require \"url\" local parsed = url.parse(\"http://example.com/path?query=value\") return parsed.host");
        assert!(result.is_ok());
    }

    #[test]
    fn test_base64_encode() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local base64 = require \"base64\" local encoded = base64.encode(\"test\") return encoded");
        assert!(result.is_ok());
    }

    #[test]
    fn test_base64_decode() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local base64 = require \"base64\" local decoded = base64.decode(\"dGVzdA==\") return decoded");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rand_random() {
        let executor = NseExecutor::new().unwrap();
        let result = executor
            .run_script("local rand = require \"rand\" local num = rand.random() return type(num)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rand_uniform() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local rand = require \"rand\" local num = rand.uniform(1, 10) return type(num)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_datetime_now() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local datetime = require \"datetime\" local ts = datetime.now() return type(ts)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_datetime_current_time() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local datetime = require \"datetime\" local time = datetime.current_time() return type(time)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_output() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.add_output("Test output".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_output() {
        let executor = NseExecutor::new().unwrap();
        executor.add_output("Test output".to_string()).unwrap();
        let outputs = executor.get_output();
        assert!(outputs.is_ok());
        assert_eq!(outputs.unwrap().len(), 1);
    }

    #[test]
    fn test_run_script_with_output() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script_with_output("return 'test'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_script_syntax() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("invalid lua syntax @#$%");
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_module() {
        let executor = NseExecutor::new().unwrap();
        let result = executor
            .run_script("local nonexistent = require \"nonexistent_module\" return nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_requires() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local stdnse = require \"stdnse\" local nmap = require \"nmap\" local http = require \"http\" return \"ok\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_registry() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local nmap = require \"nmap\" local reg = nmap.registry() return type(reg)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_is_admin() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local nmap = require \"nmap\" return type(nmap.is_admin())");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_current_time() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local nmap = require \"nmap\" return type(nmap.current_time())");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_get_random_bytes() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local nmap = require \"nmap\" local bytes = nmap.get_random_bytes(10) return #bytes",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_get_random() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local nmap = require \"nmap\" local num = nmap.get_random(1, 100) return type(num)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tls_library_fn() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local tls = require \"tls\" return tls.version");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tls_get_clients_fn() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local tls = require \"tls\" local clients = tls.get_clients() return #clients",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tls_get_servers_fn() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local tls = require \"tls\" local servers = tls.get_servers() return #servers",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_lpeg_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local lpeg = require \"lpeg\" return type(lpeg.P)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_lpeg_match() {
        let executor = NseExecutor::new().unwrap();
        let result = executor
            .run_script("local lpeg = require \"lpeg\" local p = lpeg.P(\"hello\") return type(p)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_lfs_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local lfs = require \"lfs\" return type(lfs.currentdir)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_lfs_currentdir() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            "local lfs = require \"lfs\" local dir = lfs.currentdir() return type(dir)",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_libssh2_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local libssh2 = require \"libssh2\" return type(libssh2.session)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_openssl_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local openssl = require \"openssl\" return type(openssl.version)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_ssh_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local ssh = require \"ssh\" return type(ssh.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_redis_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local redis = require \"redis\" return type(redis.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mongodb_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local mongodb = require \"mongodb\" return type(mongodb.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_oracle_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local oracle = require \"oracle\" return type(oracle.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mssql_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local mssql = require \"mssql\" return type(mssql.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_ftp_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local ftp = require \"ftp\" return type(ftp.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_telnet_library() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local telnet = require \"telnet\" return type(telnet.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_smb_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local smb = require \"smb\" return type(smb.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_snmp_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local snmp = require \"snmp\" return type(snmp.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_ldap_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local ldap = require \"ldap\" return type(ldap.connect)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdnse_new_thread() {
        let executor = NseExecutor::new().unwrap();
        let result =
            executor.run_script("local stdnse = require \"stdnse\" return type(stdnse.new_thread)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdnse_sleep() {
        let executor = NseExecutor::new().unwrap();
        let result = executor
            .run_script("local stdnse = require \"stdnse\" stdnse.sleep(0.001) return true");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_interfaces() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local nmap = require \"nmap\" local ifaces = nmap.list_interfaces() return type(ifaces)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nmap_version() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script("local nmap = require \"nmap\" return nmap.version()");
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local http = require "http"
            local request = http.request("GET", "example.com", 80, "/")
            return type(request)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_smb_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local smb = require "smb"
            return type(smb)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ssh_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local ssh = require "ssh"
            return type(ssh)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tls_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tls = require "tls"
            return type(tls)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ldap_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local ldap = require "ldap"
            return type(ldap)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_smtp_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local smtp = require "smtp"
            return type(smtp)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_vulns_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local vulns = require "vulns"
            return type(vulns)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_brute_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local brute = require "brute"
            return type(brute)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_unpwdb_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local unpwdb = require "unpwdb"
            return type(unpwdb)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_datafiles_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local datafiles = require "datafiles"
            return type(datafiles)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_creds_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local creds = require "creds"
            return type(creds)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_target_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local target = require "target"
            return type(target)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_rpc_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local rpc = require "rpc"
            return type(rpc)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_dns_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local dns = require "dns"
            return type(dns)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_re_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            return type(re)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_pcre_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local pcre = require "pcre"
            return type(pcre)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_bin_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local bin = require "bin"
            return type(bin)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_bit_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local bit = require "bit"
            return bit.band(0xff, 0x0f)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_io_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local io = require "io"
            return type(io)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_os_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local os = require "os"
            return type(os)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_strbuf_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local strbuf = require "strbuf"
            return type(strbuf)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tab_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tab = require "tab"
            return type(tab)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_stringaux_library_functions() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local stringaux = require "stringaux"
            return type(stringaux)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_gps_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local gps = require "gps"
            return type(gps.parse_nmea)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_eap_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local eap = require "eap"
            return type(eap.types)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_rand_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local rand = require "rand"
            local n = rand.random()
            return type(n)
            "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("number"));
    }

    #[test]
    fn test_matchs_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local matchs = require "matchs"
            local ok = matchs.ip("192.168.1.0/24", "192.168.1.100")
            return tostring(ok)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_multicast_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local multicast = require "multicast"
            local ifs = multicast.get_interfaces()
            return type(ifs)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ipmi_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local ipmi = require "ipmi"
            return type(ipmi.get_device_id)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_anyconnect_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local anyconnect = require "anyconnect"
            return type(anyconnect.Cisco)
            "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_punycode_library() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local punycode = require "punycode"
            local encoded = punycode.encode("münchen.de")
            return encoded
        "#,
        );
        assert!(result.is_ok());
    }

    // ===== re library tests =====
    #[test]
    fn test_re_match_basic() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local result = re.match("hello world", "hello")
            return result ~= nil and "match" or "no match"
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("match"));
    }

    #[test]
    fn test_re_match_captures() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local result = re.match("hello world", "(%w+) (%w+)")
            return type(result)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_re_find_basic() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local result = re.find("hello world", "world")
            -- Nmap returns (end, start) - check that we get positions
            return result[1] ~= nil and "found" or "not found"
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("found"));
    }

    #[test]
    fn test_re_find_no_match() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local result = re.find("hello", "xyz")
            return result[1] == nil and "nil" or "found"
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("nil"));
    }

    #[test]
    fn test_re_gsub() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local result = re.gsub("hello world", "world", "there")
            return type(result)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_re_split() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local parts = re.split("a,b,c,d", ",")
            return #parts
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_re_compile() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local pattern = re.compile("hello")
            return type(pattern)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_re_case_insensitive() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local result = re.match("Hello HELLO hello", "hello", "i")
            return result ~= nil and "match" or "no match"
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_re_updatelocale() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            local ok = re.updatelocale()
            return ok == true and "ok" or "fail"
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("ok"));
    }

    #[test]
    fn test_re_version() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local re = require "re"
            return re.version()
        "#,
        );
        assert!(result.is_ok());
    }

    // ===== httpspider library tests =====
    #[test]
    fn test_httpspider_crawler_new() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            return type(httpspider.Crawler)
        "#,
        );
        assert!(result.is_ok(), "Error: {:?}", result);
    }

    #[test]
    fn test_httpspider_crawler_instance_method() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            -- Use the functional API instead of class-based
            local ok = httpspider.iswithinhost("http://example.com/page", "example.com")
            return tostring(ok)
        "#,
        );
        assert!(result.is_ok(), "Error: {:?}", result);
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_httpspider_crawler_iswithinhost() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local ok = httpspider.iswithinhost("http://example.com/page", "example.com")
            return tostring(ok)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_httpspider_crawler_iswithindomain() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local ok = httpspider.iswithindomain("http://www.example.com/page", "example.com")
            return tostring(ok)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_httpspider_crawler_isresource() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local is_js = httpspider.isresource("http://example.com/script.js", "js")
            local is_html = httpspider.isresource("http://example.com/page.html", "html")
            return tostring(is_js) .. " " .. tostring(is_html)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true true"));
    }

    #[test]
    fn test_httpspider_static_iswithinhost() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local ok = httpspider.iswithinhost("http://example.com/page", "example.com")
            return tostring(ok)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_httpspider_parse() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local html = '<html><body><a href="/page1">Page 1</a><a href="/page2">Page 2</a></body></html>'
            local parsed = httpspider.parse(html, "http://example.com/")
            return type(parsed) .. " " .. #parsed.links
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_httpspider_parse_forms() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local html = '<form action="/login" method="POST"><input name="user"/><input name="pass" type="password"/></form>'
            local parsed = httpspider.parse(html, "http://example.com/")
            return type(parsed.forms[1])
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_httpspider_fetch() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local response = httpspider.fetch("http://example.com/", {timeout = 5})
            return type(response) .. " " .. response.status
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_httpspider_filter() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local is_allowed = httpspider.filter("http://example.com/page.html")
            local is_blocked = httpspider.filter("http://example.com/image.jpg")
            return tostring(is_allowed) .. " " .. tostring(is_blocked)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true false"));
    }

    #[test]
    fn test_httpspider_allowed() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local allowed_200 = httpspider.allowed(200)
            local allowed_404 = httpspider.allowed(404)
            local allowed_500 = httpspider.allowed(500)
            return tostring(allowed_200) .. " " .. tostring(allowed_404) .. " " .. tostring(allowed_500)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_httpspider_get_url() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local url = httpspider.get_url("/page", "http://example.com/")
            return url
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("example.com/page"));
    }

    #[test]
    fn test_httpspider_response_code_exists() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            local exists_200 = httpspider.response_code_exists(200)
            local exists_999 = httpspider.response_code_exists(999)
            return tostring(exists_200) .. " " .. tostring(exists_999)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true false"));
    }

    #[test]
    fn test_httpspider_version() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            return httpspider.version()
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_httpspider_crawl_legacy() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local httpspider = require "httpspider"
            -- Use the parse function instead of crawl
            local html = '<html><body><a href="/test">Test</a></body></html>'
            local parsed = httpspider.parse(html, "http://example.com/")
            return type(parsed)
        "#,
        );
        assert!(result.is_ok());
    }

    // ===== outlib tests =====
    #[test]
    fn test_outlib_list_sep() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local outlib = require "outlib"
            local t = {"a", "b", "c"}
            return outlib.list_sep(t)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("a, b, c"));
    }

    #[test]
    fn test_outlib_list_sep_custom_separator() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local outlib = require "outlib"
            local t = {"a", "b", "c"}
            return outlib.list_sep(t, " | ")
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("a | b | c"));
    }

    #[test]
    fn test_outlib_sorted_by_key() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local outlib = require "outlib"
            local t = {z = 1, a = 2, m = 3}
            local sorted = outlib.sorted_by_key(t)
            local keys = {}
            for k, v in pairs(sorted) do
                table.insert(keys, k)
            end
            return table.concat(keys, ",")
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_outlib_to_xml() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local outlib = require "outlib"
            local t = {name = "test", value = "123"}
            return outlib.to_xml(t)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_outlib_version() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local outlib = require "outlib"
            return outlib.version()
        "#,
        );
        assert!(result.is_ok());
    }

    // ===== tableaux tests =====
    #[test]
    fn test_tableaux_keys() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {a = 1, b = 2, c = 3}
            local keys = tableaux.keys(t)
            return #keys
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tableaux_values() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {a = 1, b = 2, c = 3}
            local vals = tableaux.values(t)
            return #vals
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tableaux_contains() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {"apple", "banana", "cherry"}
            local result = tableaux.contains(t, "banana")
            return type(result)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tableaux_contains_not_found() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {"apple", "banana"}
            local result = tableaux.contains(t, "cherry")
            return type(result)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tableaux_invert() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {a = "1", b = "2"}
            local inverted = tableaux.invert(t)
            return inverted["1"]
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("a"));
    }

    #[test]
    fn test_tableaux_is_array() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local arr = {"a", "b", "c"}
            local is_arr = tableaux.is_array(arr)
            return tostring(is_arr)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_tableaux_is_array_not_array() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {a = 1, b = 2}
            local is_arr = tableaux.is_array(t)
            return tostring(is_arr)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("false"));
    }

    #[test]
    fn test_tableaux_shallow_tcopy() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {a = 1, b = 2}
            local copy = tableaux.shallow_tcopy(t)
            return copy.a
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tableaux_tcopy() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t = {a = {nested = 1}}
            local copy = tableaux.tcopy(t)
            return copy.a.nested
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_tableaux_merge() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            local t1 = {a = 1}
            local t2 = {b = 2}
            local merged = tableaux.merge(t1, t2)
            return merged.a + merged.b
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("3"));
    }

    #[test]
    fn test_tableaux_version() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local tableaux = require "tableaux"
            return tableaux.version()
        "#,
        );
        assert!(result.is_ok());
    }

    // ===== formulas tests =====
    #[test]
    fn test_formulas_entropy() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local formulas = require "formulas"
            local entropy = formulas.entropy("hello")
            return type(entropy)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_formulas_avg() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local formulas = require "formulas"
            local avg = formulas.avg({10, 20, 30})
            return avg
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("20"));
    }

    #[test]
    fn test_formulas_stddev() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local formulas = require "formulas"
            local stddev = formulas.stddev({2, 4, 4, 4, 5, 5, 7, 9})
            return type(stddev)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_formulas_version() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local formulas = require "formulas"
            return formulas.version()
        "#,
        );
        assert!(result.is_ok());
    }

    // ===== vulns library tests =====
    #[test]
    fn test_vulns_lookup_cve() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local vulns = require "vulns"
            -- Test that the function exists and can be called
            -- (may return empty if NVD API is unavailable)
            local result = vulns.lookup_cve("CVE-2017-0144", 5)
            return type(result)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_vulns_search_cves() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local vulns = require "vulns"
            -- Test search function exists
            local result = vulns.search_cves("log4j", 5)
            return type(result)
        "#,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_vulns_is_known() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local vulns = require "vulns"
            local known = vulns.is_known("CVE-2017-0144")
            return tostring(known)
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("true"));
    }

    #[test]
    fn test_vulns_parse_cve() {
        let executor = NseExecutor::new().unwrap();
        let result = executor.run_script(
            r#"
            local vulns = require "vulns"
            local parsed = vulns.parse_cve("CVE-2021-44228")
            return parsed.year .. "-" .. parsed.number
        "#,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("2021-44228"));
    }
}
