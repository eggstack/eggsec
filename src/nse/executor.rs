//! NSE Executor - Lua VM setup and script execution
//!
//! This module provides the core functionality for running NSE scripts
//! using the mlua Lua interpreter (Lua 5.4).

use mlua::{Lua, Result as LuaResult, Table, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::nse::libraries::shared;

pub struct NseExecutor {
    lua: Lua,
    target: String,
    scripts_path: Arc<Mutex<Vec<PathBuf>>>,
    output: Mutex<Vec<String>>,
    registry: Mutex<HashMap<String, Value>>,
}

impl NseExecutor {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        
        let scripts_path = Arc::new(Mutex::new(vec![]));
        let output = Mutex::new(vec![]);
        let registry = Mutex::new(HashMap::new());
        
        let executor = Self {
            lua,
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
    
    fn setup_globals(&self) -> LuaResult<()> {
        let globals = self.lua.globals();
        
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
        globals.set("sybase", self.lua.create_table()?)?;
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
        globals.set("brute", self.lua.create_table()?)?;
        globals.set("datafiles", self.lua.create_table()?)?;
        globals.set("json", self.lua.create_table()?)?;
        globals.set("ssh", self.lua.create_table()?)?;
        globals.set("ssh1", self.lua.create_table()?)?;
        globals.set("ssh2", self.lua.create_table()?)?;
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
        globals.set("geoip", self.lua.create_table()?)?;
        globals.set("rpc", self.lua.create_table()?)?;
        globals.set("smb2", self.lua.create_table()?)?;
        globals.set("smbauth", self.lua.create_table()?)?;
        globals.set("match", self.lua.create_table()?)?;
        globals.set("sslv2", self.lua.create_table()?)?;
        globals.set("msrpc", self.lua.create_table()?)?;
        globals.set("ike", self.lua.create_table()?)?;
        globals.set("ipp", self.lua.create_table()?)?;
        globals.set("coap", self.lua.create_table()?)?;
        globals.set("idna", self.lua.create_table()?)?;
        globals.set("pgsql", self.lua.create_table()?)?;
        globals.set("ipOps", self.lua.create_table()?)?;
        globals.set("iax2", self.lua.create_table()?)?;
        globals.set("drda", self.lua.create_table()?)?;
        globals.set("eigrp", self.lua.create_table()?)?;
        globals.set("giop", self.lua.create_table()?)?;
        globals.set("iscsi", self.lua.create_table()?)?;
        globals.set("jdwp", self.lua.create_table()?)?;
        globals.set("rsync", self.lua.create_table()?)?;
        globals.set("socks", self.lua.create_table()?)?;
        globals.set("rtsp", self.lua.create_table()?)?;
        globals.set("tn3270", self.lua.create_table()?)?;
        globals.set("xmpp", self.lua.create_table()?)?;
        globals.set("isns", self.lua.create_table()?)?;
        globals.set("membase", self.lua.create_table()?)?;
        globals.set("bitcoin", self.lua.create_table()?)?;
        globals.set("bittorrent", self.lua.create_table()?)?;
        globals.set("cassandra", self.lua.create_table()?)?;
        globals.set("dicom", self.lua.create_table()?)?;
        globals.set("knx", self.lua.create_table()?)?;
        globals.set("multicast", self.lua.create_table()?)?;
        globals.set("nbd", self.lua.create_table()?)?;
        globals.set("natpmp", self.lua.create_table()?)?;
        globals.set("proxy", self.lua.create_table()?)?;
        globals.set("srvloc", self.lua.create_table()?)?;
        globals.set("wsdd", self.lua.create_table()?)?;
        globals.set("xdmcp", self.lua.create_table()?)?;
        globals.set("bjnp", self.lua.create_table()?)?;
        globals.set("cvs", self.lua.create_table()?)?;
        globals.set("dnssd", self.lua.create_table()?)?;
        globals.set("eap", self.lua.create_table()?)?;
        globals.set("pppoe", self.lua.create_table()?)?;
        globals.set("rpcap", self.lua.create_table()?)?;
        globals.set("rmi", self.lua.create_table()?)?;
        globals.set("ipmi", self.lua.create_table()?)?;
        globals.set("irc", self.lua.create_table()?)?;
        globals.set("versant", self.lua.create_table()?)?;
        globals.set("omp2", self.lua.create_table()?)?;
        globals.set("gps", self.lua.create_table()?)?;
        globals.set("mobileme", self.lua.create_table()?)?;
        globals.set("ls", self.lua.create_table()?)?;
        globals.set("unicode", self.lua.create_table()?)?;
        globals.set("lpeg", self.lua.create_table()?)?;
        globals.set("lfs", self.lua.create_table()?)?;
        globals.set("libssh2", self.lua.create_table()?)?;
        
        globals.set("_REQUIRE_MODULES", self.lua.create_table()?)?;
        globals.set("_SCRIPT_OUTPUT", self.lua.create_table()?)?;
        
        Ok(())
    }
    
    fn register_libraries(&self) -> LuaResult<()> {
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
        crate::nse::libraries::lpeg::register_lpeg_library(&self.lua)?;
        crate::nse::libraries::lfs::register_lfs_library(&self.lua)?;
        crate::nse::libraries::libssh2::register_libssh2_library(&self.lua)?;
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
        let module_cache: std::sync::Mutex<HashMap<String, Value>> = std::sync::Mutex::new(HashMap::new());
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
                // Try .nse first
                let script_path = base_path.join(format!("{}.nse", name));
                if script_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&script_path) {
                        // Compile and load the script
                        let load_result = lua.load(&content).eval::<Value>();
                        if load_result.is_ok() {
                            let globals = lua.globals();
                            
                            // Try to get the module from _REQUIRE_MODULES
                            if let Ok(modules) = globals.get::<Table>("_REQUIRE_MODULES") {
                                if let Ok(module) = modules.get::<Table>(name.as_str()) {
                                    let mut cache_guard = cache.lock().unwrap();
                                    cache_guard.insert(name.clone(), Value::Table(module.clone()));
                                    return Ok(Value::Table(module));
                                }
                            }
                            
                            // Try global table
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
                
                // Try .lua
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
            
            Err(mlua::Error::RuntimeError(format!("module '{}' not found", name)))
        })?;
        
        self.lua.globals().set("require", require_fn)?;
        
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
    
    pub fn set_target(&mut self, target: &str) -> Result<(), String> {
        self.target = target.to_string();
        
        self.lua.globals()
            .get::<mlua::Table>("nmap")
            .map_err(|e| e.to_string())?
            .set("target", target)
            .map_err(|e| e.to_string())
    }
    
    pub fn set_script_args(&mut self, args: &str) -> Result<(), String> {
        if args.is_empty() {
            return Ok(());
        }
        
        let stdnse = self.lua.globals()
            .get::<mlua::Table>("stdnse")
            .map_err(|e| e.to_string())?;
        
        stdnse.set("script_args", args).map_err(|e| e.to_string())?;
        
        let args_table = self.lua.create_table().map_err(|e| e.to_string())?;
        for pair in args.split(',') {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once('=') {
                args_table.set(key.trim(), value.trim()).map_err(|e| e.to_string())?;
            }
        }
        stdnse.set("args", args_table).map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    pub fn add_output(&self, output: String) -> Result<(), String> {
        let mut out = self.output.lock().map_err(|e| e.to_string())?;
        out.push(output.clone());
        
        if let Ok(script_output) = self.lua.globals().get::<mlua::Table>("_SCRIPT_OUTPUT") {
            let _ = script_output.set(out.len(), output);
        }
        
        Ok(())
    }
    
    pub fn get_output(&self) -> Result<Vec<String>, String> {
        let out = self.output.lock().map_err(|e| e.to_string())?;
        Ok(out.clone())
    }
    
    pub fn get_script_output(&self) -> Result<String, String> {
        let globals = self.lua.globals();
        
        let script_output = globals.get::<mlua::Table>("_SCRIPT_OUTPUT")
            .map_err(|e| e.to_string())?;
        
        let mut result = String::new();
        for pair in script_output.pairs::<mlua::Value, mlua::Value>() {
            if let Ok((_, v)) = pair {
                let val_str = match v {
                    mlua::Value::String(s) => s.to_string_lossy().to_string(),
                    _ => {
                        let s = v.to_string();
                        match s {
                            Ok(s) => s,
                            Err(_) => String::new(),
                        }
                    }
                };
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&val_str);
            }
        }
        
        Ok(result)
    }
    
    pub fn run_script(&self, script: &str) -> LuaResult<String> {
        match self.lua.load(script).eval::<mlua::Value>() {
            Ok(result) => {
                // Try to get any script output
                if let Ok(script_output) = self.get_script_output() {
                    if !script_output.is_empty() {
                        return Ok(script_output);
                    }
                }
                
                // Return result if it's not nil
                if !result.is_nil() {
                    return Ok(format!("{:?}", result));
                }
                
                Ok("Script executed successfully".to_string())
            }
            Err(e) => {
                // Provide more detailed error information
                let error_msg = format!("Lua script execution error: {}", e);
                Err(mlua::Error::RuntimeError(error_msg))
            }
        }
    }

    pub fn run_script_with_rules(&mut self, script: &str) -> LuaResult<(String, Vec<String>)> {
        self.lua.load(script).eval::<mlua::Value>()?;
        
        let globals = self.lua.globals();
        
        let mut outputs = Vec::new();
        
        // Execute prerule if present
        if let Ok(prerule) = globals.get::<mlua::Function>("prerule") {
            match prerule.call::<mlua::Value>(()) {
                Ok(result) => {
                    if !result.is_nil() {
                        outputs.push(format!("prerule: {:?}", result));
                    }
                }
                Err(e) => {
                    outputs.push(format!("prerule error: {}", e));
                }
            }
        }
        
        // Execute hostrule if present
        let hostrule_matched = if let Ok(hostrule) = globals.get::<mlua::Function>("hostrule") {
            let host = globals.get::<mlua::Table>("nmap")?;
            match hostrule.call::<mlua::Value>(host.clone()) {
                Ok(result) => {
                    if !result.is_nil() && result.as_boolean().unwrap_or(false) {
                        // Execute action for host
                        if let Ok(action) = globals.get::<mlua::Function>("action") {
                            match action.call::<mlua::Value>((host.clone(), self.lua.create_table()?)) {
                                Ok(val) => {
                                    if !val.is_nil() {
                                        outputs.push(format!("action: {:?}", val));
                                    }
                                }
                                Err(e) => {
                                    outputs.push(format!("action error: {}", e));
                                }
                            }
                        }
                        true
                    } else {
                        false
                    }
                }
                Err(e) => {
                    outputs.push(format!("hostrule error: {}", e));
                    false
                }
            }
        } else {
            false
        };
        
        // Execute portrule for each port if hostrule didn't match or wasn't present
        let ports = globals.get::<mlua::Table>("nmap")?.get::<mlua::Table>("_ports")?;
        let mut portrule_matched = false;
        
        for pair in ports.pairs::<String, mlua::Table>() {
            if let Ok((_key, port_info)) = pair {
                if let Ok(portrule) = globals.get::<mlua::Function>("portrule") {
                    match portrule.call::<mlua::Value>(port_info.clone()) {
                        Ok(result) => {
                            if !result.is_nil() && result.as_boolean().unwrap_or(false) {
                                // Execute action for port
                                if let Ok(action) = globals.get::<mlua::Function>("action") {
                                    let host = globals.get::<mlua::Table>("nmap")?;
                                    match action.call::<mlua::Value>((host.clone(), port_info.clone())) {
                                        Ok(val) => {
                                            if !val.is_nil() {
                                                outputs.push(format!("action: {:?}", val));
                                            }
                                        }
                                        Err(e) => {
                                            outputs.push(format!("action error: {}", e));
                                        }
                                    }
                                }
                                portrule_matched = true;
                                break;
                            }
                        }
                        Err(e) => {
                            outputs.push(format!("portrule error: {}", e));
                        }
                    }
                }
            }
        }
        
        // Execute postrule if present
        if let Ok(postrule) = globals.get::<mlua::Function>("postrule") {
            match postrule.call::<mlua::Value>(()) {
                Ok(result) => {
                    if !result.is_nil() {
                        outputs.push(format!("postrule: {:?}", result));
                    }
                }
                Err(e) => {
                    outputs.push(format!("postrule error: {}", e));
                }
            }
        }
        
        // Collect script output
        if let Ok(script_output) = self.get_script_output() {
            if !script_output.is_empty() {
                outputs.push(script_output);
            }
        }
        
        // If no rules matched and no output, return default message
        if outputs.is_empty() && !hostrule_matched && !portrule_matched {
            outputs.push("No rules matched or no output generated".to_string());
        }
        
        Ok((outputs.join("\n"), outputs))
    }

    pub fn check_portrule(&mut self, portrule: Option<&str>, port: u16, protocol: &str, state: &str, service: Option<&str>) -> LuaResult<bool> {
        let globals = self.lua.globals();
        
        // Create port info table with all available fields
        let port_table = self.lua.create_table()?;
        port_table.set("number", port)?;
        port_table.set("protocol", protocol)?;
        port_table.set("state", state)?;
        if let Some(svc) = service {
            port_table.set("service", svc)?;
        }
        
        // Try custom rule first
        if let Some(rule) = portrule {
            if !rule.is_empty() {
                if let Ok(rule_fn) = self.lua.load(rule).eval::<mlua::Function>() {
                    if let Ok(result) = rule_fn.call::<mlua::Value>(port_table.clone()) {
                        return Ok(result.as_boolean().unwrap_or(false));
                    }
                }
            }
        }
        
        // Try registered portrule function
        if let Ok(portrule_fn) = globals.get::<mlua::Function>("portrule") {
            if let Ok(result) = portrule_fn.call::<mlua::Value>(port_table) {
                return Ok(result.as_boolean().unwrap_or(false));
            }
        }
        
        // Default: no rule means all ports match
        Ok(true)
    }

    pub fn check_hostrule(&mut self, hostrule: Option<&str>) -> LuaResult<bool> {
        let globals = self.lua.globals();
        
        // Get host info from nmap table
        let host = match globals.get::<mlua::Table>("nmap") {
            Ok(nmap) => nmap,
            Err(_) => return Ok(true), // Default to match if nmap table unavailable
        };
        
        // Try custom rule first
        if let Some(rule) = hostrule {
            if !rule.is_empty() {
                if let Ok(rule_fn) = self.lua.load(rule).eval::<mlua::Function>() {
                    if let Ok(result) = rule_fn.call::<mlua::Value>(host.clone()) {
                        return Ok(result.as_boolean().unwrap_or(false));
                    }
                }
            }
        }
        
        // Try registered hostrule function
        if let Ok(hostrule_fn) = globals.get::<mlua::Function>("hostrule") {
            if let Ok(result) = hostrule_fn.call::<mlua::Value>(host) {
                return Ok(result.as_boolean().unwrap_or(false));
            }
        }
        
        // Default: no rule means host matches
        Ok(true)
    }

    pub fn get_prerule_result(&self) -> Option<String> {
        let globals = self.lua.globals();
        let prerule = globals.get::<mlua::Function>("prerule").ok()?;
        let result = prerule.call::<mlua::Value>(()).ok()?;
        Some(format!("{:?}", result))
    }

    pub fn get_postrule_result(&self) -> Option<String> {
        let globals = self.lua.globals();
        let postrule = globals.get::<mlua::Function>("postrule").ok()?;
        let result = postrule.call::<mlua::Value>(()).ok()?;
        Some(format!("{:?}", result))
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

    pub fn set_host_info(&mut self, hostname: Option<String>, ip: String, mac: Option<String>, status: Option<String>) -> Result<(), String> {
        let globals = self.lua.globals();
        let nmap = globals.get::<mlua::Table>("nmap").map_err(|e| e.to_string())?;
        
        let hostinfo = self.lua.create_table().map_err(|e| e.to_string())?;
        hostinfo.set("ip", ip).map_err(|e| e.to_string())?;
        
        if let Some(h) = hostname {
            hostinfo.set("hostname", h).map_err(|e| e.to_string())?;
        }
        
        if let Some(m) = mac {
            hostinfo.set("mac", m).map_err(|e| e.to_string())?;
        }
        
        if let Some(s) = status {
            hostinfo.set("status", s).map_err(|e| e.to_string())?;
        }
        
        nmap.set("_hostinfo", hostinfo).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    pub fn add_port(&mut self, port: u16, protocol: &str, state: &str, service: Option<String>) -> Result<(), String> {
        let globals = self.lua.globals();
        let nmap = globals.get::<mlua::Table>("nmap").map_err(|e| e.to_string())?;
        
        let ports = nmap.get::<mlua::Table>("_ports").map_err(|e| e.to_string())?;
        
        let key = format!("{}.{}.{}", self.target, port, protocol);
        
        let port_info = self.lua.create_table().map_err(|e| e.to_string())?;
        port_info.set("number", port).map_err(|e| e.to_string())?;
        port_info.set("protocol", protocol).map_err(|e| e.to_string())?;
        port_info.set("state", state).map_err(|e| e.to_string())?;
        
        if let Some(svc) = service {
            port_info.set("service", svc).map_err(|e| e.to_string())?;
        }
        
        ports.set(key, port_info).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }
    
    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn check_script_category(&self, script_name: &str, category: &str) -> bool {
        let categories = get_script_categories();
        
        if let Some(script_cats) = categories.get(script_name) {
            return script_cats.contains(&category);
        }
        
        match category {
            "default" | "safe" => true,
            _ => false,
        }
    }

    pub fn get_script_categories(&self, script_name: &str) -> Vec<String> {
        let categories = get_script_categories();
        
        if let Some(cats) = categories.get(script_name) {
            return cats.iter().map(|s| s.to_string()).collect();
        }
        
        vec!["default".to_string()]
    }

    pub fn get_category_scripts(&self, category: &str) -> Vec<String> {
        let categories = get_script_categories();
        
        let mut scripts = Vec::new();
        for (name, cats) in categories.iter() {
            if cats.contains(&category) {
                scripts.push(name.to_string());
            }
        }
        
        scripts
    }
}

fn get_script_categories() -> std::collections::HashMap<&'static str, Vec<&'static str>> {
    use std::collections::HashMap;
    let mut categories = HashMap::new();
    
    categories.insert("http-title", vec!["default", "discovery", "safe"]);
    categories.insert("http-headers", vec!["default", "discovery", "safe"]);
    categories.insert("http-methods", vec!["default", "discovery", "safe"]);
    categories.insert("http-robots.txt", vec!["discovery", "safe"]);
    categories.insert("ssh2-enum-algos", vec!["discovery", "safe"]);
    categories.insert("banner", vec!["default", "discovery"]);
    categories.insert("broadcast", vec!["broadcast"]);
    categories.insert("smb-brute", vec!["brute", "intrusive"]);
    categories.insert("http-brute", vec!["brute", "intrusive"]);
    categories.insert("ftp-brute", vec!["brute", "intrusive"]);
    categories.insert("ssh-brute", vec!["brute", "intrusive"]);
    categories.insert("mysql-enum", vec!["auth", "default"]);
    categories.insert("smb-enum-users", vec!["discovery", "safe"]);
    categories.insert("smb-enum-shares", vec!["discovery", "safe"]);
    categories.insert("vuln", vec!["vuln", "safe"]);
    categories.insert("exploit", vec!["exploit", "intrusive"]);
    categories.insert("dos", vec!["dos", "intrusive"]);
    
    categories
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
