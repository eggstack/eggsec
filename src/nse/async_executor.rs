//! Async NSE Executor - Tokio-based async Lua VM
//!
//! This module provides async functionality for running NSE scripts
//! using tokio for non-blocking I/O operations.

use mlua::{Lua, Result as LuaResult, Table, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::nse::libraries::shared;

static USE_EXISTING_RUNTIME: AtomicBool = AtomicBool::new(true);

pub fn set_use_existing_runtime(use_existing: bool) {
    USE_EXISTING_RUNTIME.store(use_existing, Ordering::SeqCst);
}

/// Async NSE Executor with tokio runtime support
pub struct AsyncNseExecutor {
    lua: Lua,
    runtime: Option<Runtime>,
    owns_runtime: bool,
    target: String,
    scripts_path: Arc<Mutex<Vec<PathBuf>>>,
    output: Mutex<Vec<String>>,
    registry: Mutex<HashMap<String, Value>>,
}

impl AsyncNseExecutor {
    /// Create a new async executor with tokio runtime
    pub fn new() -> LuaResult<Self> {
        let (runtime, owns_runtime) = if USE_EXISTING_RUNTIME.load(Ordering::SeqCst) {
            // Try to get the current runtime, create new one if not available
            match tokio::runtime::Handle::try_current() {
                Ok(_handle) => {
                    // Note: We can't use try_current() to create a Runtime,
                    // so we'll create a new multi-threaded runtime
                    let rt = Runtime::new().map_err(|e| {
                        mlua::Error::RuntimeError(format!("Failed to create tokio runtime: {}", e))
                    })?;
                    (Some(rt), true)
                }
                Err(_) => {
                    // No current runtime, create a new one
                    let rt = Runtime::new().map_err(|e| {
                        mlua::Error::RuntimeError(format!("Failed to create tokio runtime: {}", e))
                    })?;
                    (Some(rt), true)
                }
            }
        } else {
            let rt = Runtime::new().map_err(|e| {
                mlua::Error::RuntimeError(format!("Failed to create tokio runtime: {}", e))
            })?;
            (Some(rt), true)
        };

        // Create async Lua instance with tokio integration
        let lua = Lua::new();

        let scripts_path = Arc::new(Mutex::new(vec![]));
        let output = Mutex::new(vec![]);
        let registry = Mutex::new(HashMap::new());

        let executor = Self {
            lua,
            runtime,
            owns_runtime,
            target: String::new(),
            scripts_path: scripts_path.clone(),
            output,
            registry,
        };

        executor.setup_globals()?;
        executor.register_libraries()?;
        executor.setup_require(scripts_path)?;

        Ok(executor)
    }

    /// Create async executor with target
    pub fn with_target(target: &str) -> LuaResult<Self> {
        let mut executor = Self::new()?;
        executor.target = target.to_string();
        Ok(executor)
    }

    /// Create async executor with an existing runtime
    pub fn with_runtime(runtime: Runtime) -> LuaResult<Self> {
        let lua = Lua::new();

        let scripts_path = Arc::new(Mutex::new(vec![]));
        let output = Mutex::new(vec![]);
        let registry = Mutex::new(HashMap::new());

        let executor = Self {
            lua,
            runtime: Some(runtime),
            owns_runtime: false,
            target: String::new(),
            scripts_path: scripts_path.clone(),
            output,
            registry,
        };

        executor.setup_globals()?;
        executor.register_libraries()?;
        executor.setup_require(scripts_path)?;

        Ok(executor)
    }

    /// Set the target host
    pub fn set_target(&mut self, target: &str) {
        self.target = target.to_string();

        // Update nmap library with target info
        let globals = self.lua.globals();
        if let Ok(nmap) = globals.get::<Table>("nmap") {
            let _ = nmap.set("target", target);
        }
    }

    /// Get the target host
    pub fn get_target(&self) -> &str {
        &self.target
    }

    fn setup_globals(&self) -> LuaResult<()> {
        let globals = self.lua.globals();

        // Register all standard NSE libraries (same as regular executor)
        globals.set("stdnse", self.lua.create_table()?)?;
        globals.set("nmap", self.lua.create_table()?)?;
        globals.set("http", self.lua.create_table()?)?;
        globals.set("comm", self.lua.create_table()?)?;
        globals.set("sslcert", self.lua.create_table()?)?;
        globals.set("tls", self.lua.create_table()?)?;
        globals.set("shortport", self.lua.create_table()?)?;
        globals.set("socket", self.lua.create_table()?)?;
        globals.set("ssh2", self.lua.create_table()?)?;
        globals.set("ftp", self.lua.create_table()?)?;
        globals.set("smtp", self.lua.create_table()?)?;
        globals.set("mysql", self.lua.create_table()?)?;
        globals.set("postgres", self.lua.create_table()?)?;
        globals.set("pgsql", self.lua.create_table()?)?;
        globals.set("mssql", self.lua.create_table()?)?;
        globals.set("redis", self.lua.create_table()?)?;
        globals.set("mongodb", self.lua.create_table()?)?;
        globals.set("ldap", self.lua.create_table()?)?;
        globals.set("snmp", self.lua.create_table()?)?;
        globals.set("smb", self.lua.create_table()?)?;
        globals.set("rdp", self.lua.create_table()?)?;
        globals.set("vnc", self.lua.create_table()?)?;
        globals.set("ntp", self.lua.create_table()?)?;
        globals.set("memcached", self.lua.create_table()?)?;
        globals.set("imap", self.lua.create_table()?)?;
        globals.set("pop3", self.lua.create_table()?)?;
        globals.set("netbios", self.lua.create_table()?)?;
        globals.set("oracle", self.lua.create_table()?)?;
        globals.set("winrm", self.lua.create_table()?)?;
        globals.set("radius", self.lua.create_table()?)?;
        globals.set("dhcp", self.lua.create_table()?)?;
        globals.set("dhcp6", self.lua.create_table()?)?;
        globals.set("sip", self.lua.create_table()?)?;
        globals.set("tftp", self.lua.create_table()?)?;
        globals.set("upnp", self.lua.create_table()?)?;
        globals.set("tns", self.lua.create_table()?)?;
        globals.set("afp", self.lua.create_table()?)?;
        globals.set("amqp", self.lua.create_table()?)?;
        globals.set("ajp", self.lua.create_table()?)?;
        globals.set("ncp", self.lua.create_table()?)?;
        globals.set("ndmp", self.lua.create_table()?)?;
        globals.set("nrpc", self.lua.create_table()?)?;
        globals.set("citrixxml", self.lua.create_table()?)?;
        globals.set("ospf", self.lua.create_table()?)?;
        globals.set("asn1", self.lua.create_table()?)?;
        globals.set("sasl", self.lua.create_table()?)?;
        globals.set("slaxml", self.lua.create_table()?)?;
        globals.set("re", self.lua.create_table()?)?;
        globals.set("httpspider", self.lua.create_table()?)?;
        globals.set("base64", self.lua.create_table()?)?;
        globals.set("base32", self.lua.create_table()?)?;
        globals.set("datetime", self.lua.create_table()?)?;
        globals.set("rand", self.lua.create_table()?)?;
        globals.set("url", self.lua.create_table()?)?;
        globals.set("creds", self.lua.create_table()?)?;
        globals.set("openssl", self.lua.create_table()?)?;
        globals.set("pcre", self.lua.create_table()?)?;
        globals.set("io", self.lua.create_table()?)?;
        globals.set("os", self.lua.create_table()?)?;
        globals.set("unittest", self.lua.create_table()?)?;
        globals.set("target", self.lua.create_table()?)?;
        globals.set("strbuf", self.lua.create_table()?)?;
        globals.set("tab", self.lua.create_table()?)?;
        globals.set("stringaux", self.lua.create_table()?)?;
        globals.set("bin", self.lua.create_table()?)?;
        globals.set("bit", self.lua.create_table()?)?;
        globals.set("vulns", self.lua.create_table()?)?;
        globals.set("unpwdb", self.lua.create_table()?)?;
        globals.set("datafiles", self.lua.create_table()?)?;
        globals.set("json", self.lua.create_table()?)?;
        globals.set("ssh", self.lua.create_table()?)?;
        globals.set("dns", self.lua.create_table()?)?;
        globals.set("http2", self.lua.create_table()?)?;
        globals.set("elasticsearch", self.lua.create_table()?)?;
        globals.set("kafka", self.lua.create_table()?)?;
        globals.set("mqtt", self.lua.create_table()?)?;
        globals.set("websocket", self.lua.create_table()?)?;
        globals.set("telnet", self.lua.create_table()?)?;
        globals.set("sftp", self.lua.create_table()?)?;
        globals.set("whois", self.lua.create_table()?)?;
        globals.set("finger", self.lua.create_table()?)?;
        globals.set("stun", self.lua.create_table()?)?;
        globals.set("packet", self.lua.create_table()?)?;
        globals.set("nsedebug", self.lua.create_table()?)?;
        globals.set("strict", self.lua.create_table()?)?;

        globals.set("_REQUIRE_MODULES", self.lua.create_table()?)?;
        globals.set("_SCRIPT_OUTPUT", self.lua.create_table()?)?;

        Ok(())
    }

    fn register_libraries(&self) -> LuaResult<()> {
        // Register all libraries (same as regular executor)
        crate::nse::libraries::stdnse::register_stdlib(&self.lua)?;
        crate::nse::libraries::nmap::register_nmap_library(&self.lua)?;
        crate::nse::libraries::http::register_http_library(&self.lua)?;
        crate::nse::libraries::comm::register_comm_library(&self.lua)?;
        crate::nse::libraries::sslcert::register_sslcert_library(&self.lua)?;
        crate::nse::libraries::tls::register_tls_library(&self.lua)?;
        crate::nse::libraries::shortport::register_shortport_library(&self.lua)?;
        crate::nse::libraries::socket::register_socket_library(&self.lua)?;
        crate::nse::libraries::ssh2::register_ssh2_library(&self.lua)?;
        crate::nse::libraries::ftp::register_ftp_library(&self.lua)?;
        crate::nse::libraries::smtp::register_smtp_library(&self.lua)?;
        crate::nse::libraries::mysql::register_mysql_library(&self.lua)?;
        crate::nse::libraries::postgres::register_postgres_library(&self.lua)?;
        crate::nse::libraries::mssql::register_mssql_library(&self.lua)?;
        crate::nse::libraries::redis::register_redis_library(&self.lua)?;
        crate::nse::libraries::mongodb::register_mongodb_library(&self.lua)?;
        crate::nse::libraries::ldap::register_ldap_library(&self.lua)?;
        crate::nse::libraries::snmp::register_snmp_library(&self.lua)?;
        crate::nse::libraries::smb::register_smb_library(&self.lua)?;
        crate::nse::libraries::smb2::register_smb2_library(&self.lua)?;
        crate::nse::libraries::smbauth::register_smbauth_library(&self.lua)?;
        crate::nse::libraries::rdp::register_rdp_library(&self.lua)?;
        crate::nse::libraries::vnc::register_vnc_library(&self.lua)?;
        crate::nse::libraries::ntp::register_ntp_library(&self.lua)?;
        crate::nse::libraries::memcached::register_memcached_library(&self.lua)?;
        crate::nse::libraries::imap::register_imap_library(&self.lua)?;
        crate::nse::libraries::pop3::register_pop3_library(&self.lua)?;
        crate::nse::libraries::netbios::register_netbios_library(&self.lua)?;
        crate::nse::libraries::oracle::register_oracle_library(&self.lua)?;
        crate::nse::libraries::winrm::register_winrm_library(&self.lua)?;
        crate::nse::libraries::radius::register_radius_library(&self.lua)?;
        crate::nse::libraries::dhcp::register_dhcp_library(&self.lua)?;
        crate::nse::libraries::dhcp6::register_dhcp6_library(&self.lua)?;
        crate::nse::libraries::sip::register_sip_library(&self.lua)?;
        crate::nse::libraries::tftp::register_tftp_library(&self.lua)?;
        crate::nse::libraries::upnp::register_upnp_library(&self.lua)?;
        crate::nse::libraries::tns::register_tns_library(&self.lua)?;
        crate::nse::libraries::afp::register_afp_library(&self.lua)?;
        crate::nse::libraries::amqp::register_amqp_library(&self.lua)?;
        crate::nse::libraries::ajp::register_ajp_library(&self.lua)?;
        crate::nse::libraries::ncp::register_ncp_library(&self.lua)?;
        crate::nse::libraries::ndmp::register_ndmp_library(&self.lua)?;
        crate::nse::libraries::nrpc::register_nrpc_library(&self.lua)?;
        crate::nse::libraries::citrixxml::register_citrixxml_library(&self.lua)?;
        crate::nse::libraries::ospf::register_ospf_library(&self.lua)?;
        crate::nse::libraries::asn1::register_asn1_library(&self.lua)?;
        crate::nse::libraries::sasl::register_sasl_library(&self.lua)?;
        crate::nse::libraries::slaxml::register_slaxml_library(&self.lua)?;
        crate::nse::libraries::re::register_re_library(&self.lua)?;
        crate::nse::libraries::httpspider::register_httpspider_library(&self.lua)?;
        crate::nse::libraries::base64::register_base64_library(&self.lua)?;
        crate::nse::libraries::base32::register_base32_library(&self.lua)?;
        crate::nse::libraries::datetime::register_datetime_library(&self.lua);
        crate::nse::libraries::rand::register_rand_library(&self.lua);
        crate::nse::libraries::url::register_url_library(&self.lua)?;
        crate::nse::libraries::creds::register_creds_library(&self.lua)?;
        crate::nse::libraries::openssl::register_openssl_library(&self.lua)?;
        crate::nse::libraries::pcre::register_pcre_library(&self.lua)?;
        crate::nse::libraries::io::register_io_library(&self.lua)?;
        crate::nse::libraries::os::register_os_library(&self.lua)?;
        crate::nse::libraries::unittest::register_unittest_library(&self.lua)?;
        crate::nse::libraries::target::register_target_library(&self.lua)?;
        crate::nse::libraries::strbuf::register_strbuf_library(&self.lua)?;
        crate::nse::libraries::tab::register_tab_library(&self.lua)?;
        crate::nse::libraries::stringaux::register_stringaux_library(&self.lua)?;
        crate::nse::libraries::bin::register_bin_library(&self.lua)?;
        crate::nse::libraries::bit::register_bit_library(&self.lua)?;
        crate::nse::libraries::vulns::register_vulns_library(&self.lua)?;
        crate::nse::libraries::unpwdb::register_unpwdb_library(&self.lua)?;
        crate::nse::libraries::brute::register_brute_library(&self.lua)?;
        crate::nse::libraries::datafiles::register_datafiles_library(&self.lua)?;
        crate::nse::libraries::json::register_json_library(&self.lua)?;
        crate::nse::libraries::ssh::register_ssh_library(&self.lua)?;
        crate::nse::libraries::dns::register_dns_library(&self.lua)?;
        crate::nse::libraries::http2::register_http2_library(&self.lua)?;
        crate::nse::libraries::httppipeline::register_httppipeline_library(&self.lua)?;
        crate::nse::libraries::geoip::register_geoip_library(&self.lua)?;
        crate::nse::libraries::rpc::register_rpc_library(&self.lua)?;
        crate::nse::libraries::ssh1::register_ssh1_library(&self.lua)?;
        crate::nse::libraries::sslv2::register_sslv2_library(&self.lua)?;
        crate::nse::libraries::msrpc::register_msrpc_library(&self.lua)?;
        crate::nse::libraries::ike::register_ike_library(&self.lua)?;
        crate::nse::libraries::ipp::register_ipp_library(&self.lua)?;
        crate::nse::libraries::coap::register_coap_library(&self.lua)?;
        crate::nse::libraries::idna::register_idna_library(&self.lua)?;
        crate::nse::libraries::pgsql::register_pgsql_library(&self.lua)?;
        crate::nse::libraries::ipops::register_ipops_library(&self.lua)?;
        crate::nse::libraries::iax2::register_iax2_library(&self.lua)?;
        crate::nse::libraries::drda::register_drda_library(&self.lua)?;
        crate::nse::libraries::eigrp::register_eigrp_library(&self.lua)?;
        crate::nse::libraries::giop::register_giop_library(&self.lua)?;
        crate::nse::libraries::iscsi::register_iscsi_library(&self.lua)?;
        crate::nse::libraries::jdwp::register_jdwp_library(&self.lua)?;
        crate::nse::libraries::rsync::register_rsync_library(&self.lua)?;
        crate::nse::libraries::socks::register_socks_library(&self.lua)?;
        crate::nse::libraries::rtsp::register_rtsp_library(&self.lua)?;
        crate::nse::libraries::tn3270::register_tn3270_library(&self.lua)?;
        crate::nse::libraries::xmpp::register_xmpp_library(&self.lua)?;
        crate::nse::libraries::isns::register_isns_library(&self.lua)?;
        crate::nse::libraries::membase::register_membase_library(&self.lua)?;
        crate::nse::libraries::bitcoin::register_bitcoin_library(&self.lua)?;
        crate::nse::libraries::bittorrent::register_bittorrent_library(&self.lua)?;
        crate::nse::libraries::cassandra::register_cassandra_library(&self.lua)?;
        crate::nse::libraries::dicom::register_dicom_library(&self.lua)?;
        crate::nse::libraries::knx::register_knx_library(&self.lua)?;
        crate::nse::libraries::multicast::register_multicast_library(&self.lua)?;
        crate::nse::libraries::nbd::register_nbd_library(&self.lua)?;
        crate::nse::libraries::natpmp::register_natpmp_library(&self.lua)?;
        crate::nse::libraries::proxy::register_proxy_library(&self.lua)?;
        crate::nse::libraries::srvloc::register_srvloc_library(&self.lua)?;
        crate::nse::libraries::wsdd::register_wsdd_library(&self.lua)?;
        crate::nse::libraries::xdmcp::register_xdmcp_library(&self.lua)?;
        crate::nse::libraries::bjnp::register_bjnp_library(&self.lua)?;
        crate::nse::libraries::cvs::register_cvs_library(&self.lua)?;
        crate::nse::libraries::dnssd::register_dnssd_library(&self.lua)?;
        crate::nse::libraries::eap::register_eap_library(&self.lua)?;
        crate::nse::libraries::pppoe::register_pppoe_library(&self.lua)?;
        crate::nse::libraries::rpcap::register_rpcap_library(&self.lua)?;
        crate::nse::libraries::rmi::register_rmi_library(&self.lua)?;
        crate::nse::libraries::ipmi::register_ipmi_library(&self.lua)?;
        crate::nse::libraries::irc::register_irc_library(&self.lua)?;
        crate::nse::libraries::versant::register_versant_library(&self.lua)?;
        crate::nse::libraries::omp2::register_omp2_library(&self.lua)?;
        crate::nse::libraries::gps::register_gps_library(&self.lua)?;
        crate::nse::libraries::mobileme::register_mobileme_library(&self.lua)?;
        crate::nse::libraries::ls::register_ls_library(&self.lua)?;
        crate::nse::libraries::unicode::register_unicode_library(&self.lua)?;
        crate::nse::libraries::bits::register_bits_library(&self.lua)?;
        crate::nse::libraries::formulas::register_formulas_library(&self.lua)?;
        crate::nse::libraries::anyconnect::register_anyconnect_library(&self.lua)?;
        crate::nse::libraries::iec61850mms::register_iec61850mms_library(&self.lua)?;
        crate::nse::libraries::informix::register_informix_library(&self.lua)?;
        crate::nse::libraries::libssh2_utility::register_libssh2_utility_library(&self.lua)?;
        crate::nse::libraries::matchs::register_matchs_library(&self.lua)?;
        crate::nse::libraries::lpeg_utility::register_lpeg_utility_library(&self.lua)?;
        crate::nse::libraries::msrpcperformance::register_msrpcperformance_library(&self.lua)?;
        crate::nse::libraries::msrpctypes::register_msrpctypes_library(&self.lua)?;
        crate::nse::libraries::oops::register_oops_library(&self.lua)?;
        crate::nse::libraries::outlib::register_outlib_library(&self.lua)?;
        crate::nse::libraries::punycode::register_punycode_library(&self.lua)?;
        crate::nse::libraries::tableaux::register_tableaux_library(&self.lua)?;
        crate::nse::libraries::vuzedht::register_vuzedht_library(&self.lua)?;
        crate::nse::libraries::listop::register_listop_library(&self.lua)?;
        crate::nse::libraries::zlib::register_zlib_library(&self.lua)?;

        self.sync_require_modules()
    }

    fn sync_require_modules(&self) -> LuaResult<()> {
        shared::sync_require_modules(&self.lua)
    }

    fn setup_require(&self, _scripts_path: Arc<Mutex<Vec<PathBuf>>>) -> LuaResult<()> {
        let scripts_path = self.scripts_path.clone();

        // Create a cache for compiled modules
        let module_cache: std::sync::Mutex<HashMap<String, Value>> =
            std::sync::Mutex::new(HashMap::new());
        let cache = Arc::new(module_cache);

        let require_fn = self.lua.create_function(move |lua, name: String| {
            // Check module cache first
            {
                let cache_guard = cache.lock().unwrap();
                if let Some(cached) = cache_guard.get(&name) {
                    return Ok(cached.clone());
                }
            }

            let globals = lua.globals();

            // Check if already loaded in _REQUIRE_MODULES
            if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                if let Ok(module) = modules.get::<Table>(name.as_str()) {
                    let mut cache_guard = cache.lock().unwrap();
                    cache_guard.insert(name.clone(), Value::Table(module.clone()));
                    return Ok(Value::Table(module));
                }
            }

            // Check global table directly
            if let Ok(module) = globals.get::<Table>(name.as_str()) {
                if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                    let _ = modules.set(name.clone(), module.clone());
                }
                let mut cache_guard = cache.lock().unwrap();
                cache_guard.insert(name.clone(), Value::Table(module.clone()));
                return Ok(Value::Table(module));
            }

            // Try to load from file
            let path_guard = scripts_path.lock().unwrap();
            for base_path in path_guard.iter() {
                let script_path = base_path.join(format!("{}.nse", name));
                if script_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&script_path) {
                        let load_result = lua.load(&content).eval::<Value>();
                        if load_result.is_ok() {
                            let globals = lua.globals();

                            if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                                if let Ok(module) = modules.get::<Table>(name.as_str()) {
                                    let mut cache_guard = cache.lock().unwrap();
                                    cache_guard.insert(name.clone(), Value::Table(module.clone()));
                                    return Ok(Value::Table(module));
                                }
                            }

                            if let Ok(module) = globals.get::<Table>(name.as_str()) {
                                if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                                    let _ = modules.set(name.clone(), module.clone());
                                }
                                let mut cache_guard = cache.lock().unwrap();
                                cache_guard.insert(name.clone(), Value::Table(module.clone()));
                                return Ok(Value::Table(module));
                            }
                        }
                    }
                }

                let lua_path = base_path.join(format!("{}.lua", name));
                if lua_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&lua_path) {
                        let load_result = lua.load(&content).eval::<Value>();
                        if load_result.is_ok() {
                            let globals = lua.globals();

                            if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                                if let Ok(module) = modules.get::<Table>(name.as_str()) {
                                    let mut cache_guard = cache.lock().unwrap();
                                    cache_guard.insert(name.clone(), Value::Table(module.clone()));
                                    return Ok(Value::Table(module));
                                }
                            }

                            if let Ok(module) = globals.get::<Table>(name.as_str()) {
                                if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                                    let _ = modules.set(name.clone(), module.clone());
                                }
                                let mut cache_guard = cache.lock().unwrap();
                                cache_guard.insert(name.clone(), Value::Table(module.clone()));
                                return Ok(Value::Table(module));
                            }
                        }
                    }
                }
            }

            drop(path_guard);

            Err(mlua::Error::RuntimeError(format!(
                "module '{}' not found",
                name
            )))
        })?;

        self.lua.globals().set("require", require_fn)?;

        Ok(())
    }

    /// Add a scripts path
    pub fn add_scripts_path(&self, path: PathBuf) {
        if let Ok(mut paths) = self.scripts_path.lock() {
            paths.push(path);
        }
    }

    /// Run an NSE script (blocking)
    pub fn run_script(&self, script: &str) -> LuaResult<String> {
        self.lua.load(script).eval::<Value>()?;

        let globals = self.lua.globals();
        if let Ok(output) = globals.get::<Table>("_SCRIPT_OUTPUT") {
            // Return serialized output
            return Ok(format!("{:?}", output));
        }

        Ok("Script executed successfully".to_string())
    }

    /// Get access to the Lua instance for custom operations
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Get access to the tokio runtime for async operations
    pub fn runtime(&self) -> Option<&Runtime> {
        self.runtime.as_ref()
    }
}

impl Default for AsyncNseExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create async NSE executor")
    }
}
