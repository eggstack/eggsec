use rustc_hash::FxHashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct WafSignature {
    pub name: String,
    pub headers: Vec<String>,
    pub cookies: Vec<String>,
    pub body_patterns: Vec<String>,
    pub ip_ranges: Vec<String>,
}

static WAF_SIGNATURES: LazyLock<FxHashMap<String, WafSignature>> = LazyLock::new(|| {
    let mut signatures = FxHashMap::default();

    signatures.insert(
        "cloudflare".to_string(),
        WafSignature {
            name: "Cloudflare".to_string(),
            headers: vec![
                "cf-ray".to_string(),
                "cf-cache-status".to_string(),
                "cloudflare".to_string(),
                "cf-connecting-ip".to_string(),
                "cf-ipcountry".to_string(),
                "cf-request-id".to_string(),
                "cf-worker".to_string(),
            ],
            cookies: vec!["__cfduid".to_string()],
            body_patterns: vec!["cloudflare".to_string(), "cf-browser-verify".to_string()],
            ip_ranges: vec![
                "104.16.0.0/12".to_string(),
                "172.64.0.0/13".to_string(),
                "173.245.48.0/20".to_string(),
                "103.21.244.0/22".to_string(),
                "103.22.200.0/22".to_string(),
                "103.31.4.0/22".to_string(),
                "141.101.64.0/18".to_string(),
                "108.162.192.0/18".to_string(),
                "190.93.240.0/20".to_string(),
                "188.114.96.0/20".to_string(),
                "197.234.240.0/22".to_string(),
                "198.41.128.0/17".to_string(),
                "162.158.0.0/15".to_string(),
                "131.0.72.0/22".to_string(),
            ],
        },
    );

    signatures.insert(
        "akamai".to_string(),
        WafSignature {
            name: "Akamai".to_string(),
            headers: vec![
                "x-akamai-transformed".to_string(),
                "akamai".to_string(),
                "akamaiedge".to_string(),
                "akamai-ghost".to_string(),
                "x-akamai-request-id".to_string(),
                "akamai-origin-hop".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["akamai".to_string()],
            ip_ranges: vec![
                "23.0.0.0/12".to_string(),
                "23.192.0.0/11".to_string(),
                "2.16.0.0/13".to_string(),
                "184.24.0.0/13".to_string(),
                "184.50.0.0/15".to_string(),
                "184.84.0.0/14".to_string(),
                "23.32.0.0/11".to_string(),
                "72.246.0.0/15".to_string(),
                "88.221.0.0/16".to_string(),
                "96.6.0.0/15".to_string(),
                "104.64.0.0/10".to_string(),
            ],
        },
    );

    signatures.insert(
        "aws_waf".to_string(),
        WafSignature {
            name: "AWS WAF".to_string(),
            headers: vec![
                "x-amz-cf-pop".to_string(),
                "x-amz-cf-id".to_string(),
                "x-amzn-requestid".to_string(),
                "x-amz-request-id".to_string(),
                "awselb".to_string(),
                "x-amzn-trace-id".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["aws".to_string(), "amazon".to_string()],
            ip_ranges: vec![
                "3.0.0.0/9".to_string(),
                "13.0.0.0/8".to_string(),
                "52.0.0.0/8".to_string(),
                "54.0.0.0/8".to_string(),
                "15.0.0.0/8".to_string(),
                "18.0.0.0/8".to_string(),
                "35.0.0.0/8".to_string(),
                "44.0.0.0/8".to_string(),
            ],
        },
    );

    signatures.insert(
        "azure_waf".to_string(),
        WafSignature {
            name: "Azure WAF".to_string(),
            headers: vec![
                "x-azure-ref".to_string(),
                "x-azure-origin".to_string(),
                "x-azure-fdid".to_string(),
                "x-ms-request-id".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["azure".to_string(), "microsoft".to_string()],
            ip_ranges: vec![
                "20.0.0.0/8".to_string(),
                "40.0.0.0/8".to_string(),
                "104.40.0.0/12".to_string(),
                "13.64.0.0/11".to_string(),
            ],
        },
    );

    signatures.insert(
        "google_cloud_armor".to_string(),
        WafSignature {
            name: "Google Cloud Armor".to_string(),
            headers: vec![
                "x-google-cloud-armor".to_string(),
                "x-goog-request-reason".to_string(),
                "gws".to_string(),
                "x-cloud-trace-context".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["google".to_string()],
            ip_ranges: vec![
                "8.0.0.0/8".to_string(),
                "34.0.0.0/8".to_string(),
                "35.0.0.0/8".to_string(),
                "104.0.0.0/8".to_string(),
                "107.178.0.0/15".to_string(),
                "108.170.0.0/15".to_string(),
            ],
        },
    );

    signatures.insert(
        "fastly".to_string(),
        WafSignature {
            name: "Fastly".to_string(),
            headers: vec![
                "fastly".to_string(),
                "x-served-by".to_string(),
                "x-cache".to_string(),
                "x-timer".to_string(),
                "fastly-debug-digest".to_string(),
                "x-fastly-request-id".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["fastly".to_string()],
            ip_ranges: vec![
                "23.235.32.0/20".to_string(),
                "43.249.72.0/22".to_string(),
                "104.156.80.0/20".to_string(),
                "146.75.0.0/16".to_string(),
                "151.101.0.0/16".to_string(),
                "157.52.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "imperva".to_string(),
        WafSignature {
            name: "Imperva".to_string(),
            headers: vec![
                "imperva".to_string(),
                "x-iinfo".to_string(),
                "x-imperva-client".to_string(),
                "x-imperva-backend".to_string(),
            ],
            cookies: vec!["incap_ses".to_string(), "visid_incap".to_string()],
            body_patterns: vec!["imperva".to_string(), "incapsula".to_string()],
            ip_ranges: vec![
                "199.83.128.0/21".to_string(),
                "198.143.32.0/19".to_string(),
                "45.60.0.0/16".to_string(),
                "45.223.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "sucuri".to_string(),
        WafSignature {
            name: "Sucuri".to_string(),
            headers: vec![
                "sucuri".to_string(),
                "x-sucuri-id".to_string(),
                "x-sucuri-cache".to_string(),
                "cloudproxy".to_string(),
                "x-sucuri-block".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["sucuri".to_string()],
            ip_ranges: vec![
                "192.124.249.0/24".to_string(),
                "192.161.0.0/24".to_string(),
                "192.169.0.0/16".to_string(),
                "198.58.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "cloudfront".to_string(),
        WafSignature {
            name: "CloudFront".to_string(),
            headers: vec![
                "cloudfront".to_string(),
                "x-amz-cf-id".to_string(),
                "x-amz-cf-pop".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["cloudfront".to_string()],
            ip_ranges: vec![
                "13.32.0.0/15".to_string(),
                "13.54.0.0/15".to_string(),
                "13.224.0.0/14".to_string(),
                "52.46.0.0/18".to_string(),
                "52.84.0.0/15".to_string(),
                "54.182.0.0/16".to_string(),
                "54.192.0.0/16".to_string(),
                "54.230.0.0/16".to_string(),
                "99.84.0.0/16".to_string(),
                "143.204.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "f5_big_ip".to_string(),
        WafSignature {
            name: "F5 BIG-IP".to_string(),
            headers: vec![
                "big-ip".to_string(),
                "f5".to_string(),
                "x-waf-event-info".to_string(),
                "bigipserver".to_string(),
            ],
            cookies: vec!["BIGipServer".to_string()],
            body_patterns: vec!["f5".to_string(), "big-ip".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "barracuda".to_string(),
        WafSignature {
            name: "Barracuda".to_string(),
            headers: vec![
                "barracuda".to_string(),
                "barra".to_string(),
                "x-barracuda".to_string(),
                "x-barracuda-appliance".to_string(),
            ],
            cookies: vec!["barra".to_string()],
            body_patterns: vec!["barracuda".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "fortinet".to_string(),
        WafSignature {
            name: "Fortinet".to_string(),
            headers: vec![
                "fortinet".to_string(),
                "fortigate".to_string(),
                "x-fortigate-hostname".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["fortigate".to_string(), "fortinet".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "citrix_netscaler".to_string(),
        WafSignature {
            name: "Citrix NetScaler".to_string(),
            headers: vec![
                "citrix".to_string(),
                "x-citrix".to_string(),
                "ns_af".to_string(),
                "x-citrix-appliance".to_string(),
            ],
            cookies: vec!["citrix".to_string(), "netscaler".to_string()],
            body_patterns: vec!["citrix".to_string(), "netscaler".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "modsecurity".to_string(),
        WafSignature {
            name: "ModSecurity".to_string(),
            headers: vec!["mod_security".to_string(), "modsecurity".to_string()],
            cookies: vec![],
            body_patterns: vec![
                "mod_security".to_string(),
                "modsecurity".to_string(),
                "web application firewall".to_string(),
                "not acceptable".to_string(),
            ],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "wordfence".to_string(),
        WafSignature {
            name: "Wordfence".to_string(),
            headers: vec![
                "wordfence".to_string(),
                "x-wordfence-blocked".to_string(),
                "x-wordfence-firewall".to_string(),
            ],
            cookies: vec!["wordfence".to_string()],
            body_patterns: vec!["wordfence".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "datadome".to_string(),
        WafSignature {
            name: "DataDome".to_string(),
            headers: vec![
                "datadome".to_string(),
                "x-datadome".to_string(),
                "x-datadome-client".to_string(),
            ],
            cookies: vec!["dd_".to_string(), "datadome".to_string()],
            body_patterns: vec!["datadome".to_string()],
            ip_ranges: vec![
                "35.180.0.0/16".to_string(),
                "52.47.0.0/16".to_string(),
                "54.93.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "perimeterx".to_string(),
        WafSignature {
            name: "PerimeterX".to_string(),
            headers: vec![
                "perimeterx".to_string(),
                "x-px".to_string(),
                "x-px-authorization".to_string(),
            ],
            cookies: vec!["_px".to_string()],
            body_patterns: vec!["perimeterx".to_string()],
            ip_ranges: vec![
                "52.28.0.0/16".to_string(),
                "52.57.0.0/16".to_string(),
                "52.208.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "nginx".to_string(),
        WafSignature {
            name: "Nginx".to_string(),
            headers: vec![
                "nginx".to_string(),
                "x-nginx".to_string(),
                "x-accel".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["nginx".to_string(), "403 forbidden".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "traefik".to_string(),
        WafSignature {
            name: "Traefik".to_string(),
            headers: vec!["traefik".to_string(), "x-traefik".to_string()],
            cookies: vec![],
            body_patterns: vec!["traefik".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "kong".to_string(),
        WafSignature {
            name: "Kong".to_string(),
            headers: vec![
                "kong".to_string(),
                "x-kong-upstream-latency".to_string(),
                "x-kong-proxy".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["kong".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "varnish".to_string(),
        WafSignature {
            name: "Varnish".to_string(),
            headers: vec!["varnish".to_string(), "x-varnish".to_string()],
            cookies: vec![],
            body_patterns: vec!["varnish".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "radware_waf".to_string(),
        WafSignature {
            name: "Radware".to_string(),
            headers: vec![
                "radware".to_string(),
                "x-radware-waf".to_string(),
                "x-radware-request-id".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["radware".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "signal_sciences".to_string(),
        WafSignature {
            name: "Signal Sciences".to_string(),
            headers: vec![
                "sigsci-waf".to_string(),
                "x-sigsci-requestid".to_string(),
                "x-sigsci-agentresponse".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["signal sciences".to_string()],
            ip_ranges: vec![
                "52.4.0.0/16".to_string(),
                "52.5.0.0/16".to_string(),
                "54.85.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "wallarm_waf".to_string(),
        WafSignature {
            name: "Wallarm".to_string(),
            headers: vec![
                "wallarm".to_string(),
                "x-wallarm-mode".to_string(),
                "x-wallarm-request-id".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["wallarm".to_string()],
            ip_ranges: vec![
                "34.102.136.180/32".to_string(),
                "35.235.66.155/32".to_string(),
                "104.199.0.0/16".to_string(),
            ],
        },
    );

    signatures.insert(
        "reblaze".to_string(),
        WafSignature {
            name: "Reblaze".to_string(),
            headers: vec!["reblaze".to_string(), "x-reblaze".to_string()],
            cookies: vec![],
            body_patterns: vec!["reblaze".to_string()],
            ip_ranges: vec!["45.134.140.0/22".to_string(), "185.229.0.0/22".to_string()],
        },
    );

    signatures.insert(
        "f5_bigip_asm".to_string(),
        WafSignature {
            name: "F5 BIG-IP Advanced WAF".to_string(),
            headers: vec![
                "x-cnection".to_string(),
                "x-info".to_string(),
                "x-iinfo".to_string(),
                "bigip".to_string(),
                "ts".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec![
                "the requested url was rejected".to_string(),
                "please contact the web site administrator".to_string(),
                "support id".to_string(),
                "big-ip".to_string(),
                "f5 networks".to_string(),
            ],
            ip_ranges: vec![
                "205.210.0.0/15".to_string(),
                "207.210.0.0/15".to_string(),
                "63.92.0.0/15".to_string(),
                "67.20.0.0/15".to_string(),
            ],
        },
    );

    signatures.insert(
        "palo_alto".to_string(),
        WafSignature {
            name: "Palo Alto".to_string(),
            headers: vec![
                "x-pan-a".to_string(),
                "x-paloalto".to_string(),
                "x-fireeye".to_string(),
                "infoblox".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec![
                "palo alto networks".to_string(),
                "threat signature".to_string(),
                "malware".to_string(),
                "blocked by palo alto".to_string(),
            ],
            ip_ranges: vec!["198.92.0.0/15".to_string(), "199.92.0.0/15".to_string()],
        },
    );

    signatures.insert(
        "qrator".to_string(),
        WafSignature {
            name: "Qrator".to_string(),
            headers: vec!["x-qrator".to_string(), "qrator".to_string()],
            cookies: vec!["qr".to_string()],
            body_patterns: vec!["qrator".to_string(), "blocked by qrator".to_string()],
            ip_ranges: vec![
                "185.71.64.0/22".to_string(),
                "185.71.68.0/22".to_string(),
                "188.42.128.0/17".to_string(),
            ],
        },
    );

    signatures.insert(
        "imunify360".to_string(),
        WafSignature {
            name: "Imunify360".to_string(),
            headers: vec!["imunify".to_string(), "x-imunify".to_string()],
            cookies: vec![],
            body_patterns: vec![
                "imunify".to_string(),
                "captcha".to_string(),
                "human verification".to_string(),
                "attacked".to_string(),
                "shield".to_string(),
            ],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "siteguard".to_string(),
        WafSignature {
            name: "SiteGuard".to_string(),
            headers: vec!["siteguard".to_string(), "x-siteguard".to_string()],
            cookies: vec![],
            body_patterns: vec!["siteguard".to_string(), "powered by".to_string()],
            ip_ranges: vec![],
        },
    );

    signatures.insert(
        "stackpath_waf".to_string(),
        WafSignature {
            name: "StackPath WAF".to_string(),
            headers: vec![
                "x-edge".to_string(),
                "x-cache".to_string(),
                "x-srv".to_string(),
                "x-sh".to_string(),
            ],
            cookies: vec![],
            body_patterns: vec!["stackpath".to_string(), "edge".to_string()],
            ip_ranges: vec![
                "151.236.0.0/18".to_string(),
                "178.255.0.0/18".to_string(),
                "185.94.0.0/18".to_string(),
            ],
        },
    );

    signatures.insert(
        "humanity".to_string(),
        WafSignature {
            name: "Humanity".to_string(),
            headers: vec!["x-hpx".to_string(), "x-hx".to_string()],
            cookies: vec!["px".to_string(), "_px".to_string()],
            body_patterns: vec![
                "humanity".to_string(),
                "blocked by security".to_string(),
                "human verification".to_string(),
            ],
            ip_ranges: vec!["23.14.0.0/15".to_string(), "34.100.0.0/15".to_string()],
        },
    );

    signatures.insert(
        "datadog".to_string(),
        WafSignature {
            name: "Datadog".to_string(),
            headers: vec!["x-datadog".to_string(), "datadog".to_string()],
            cookies: vec![],
            body_patterns: vec!["datadog".to_string(), "security policy".to_string()],
            ip_ranges: vec!["3.0.0.0/8".to_string()],
        },
    );

    signatures.insert(
        "denied_by_waf".to_string(),
        WafSignature {
            name: "Generic WAF Block".to_string(),
            headers: vec![],
            cookies: vec![],
            body_patterns: vec![
                "access denied".to_string(),
                "blocked".to_string(),
                "firewall".to_string(),
                "waf".to_string(),
                "forbidden".to_string(),
                "request rejected".to_string(),
                "security".to_string(),
                "unauthorized access".to_string(),
                "your request has been blocked".to_string(),
                "request blocked".to_string(),
            ],
            ip_ranges: vec![],
        },
    );

    signatures
});

pub fn get_waf_signatures() -> &'static FxHashMap<String, WafSignature> {
    &WAF_SIGNATURES
}
