use crate::tui::tabs::HistoryTab;

impl HistoryTab {
    pub fn add_load_test_result(
        &mut self,
        target: &str,
        total_requests: u64,
        successful: u64,
        failed: u64,
        rps: f64,
        mean_latency: f64,
    ) {
        let summary = format!(
            "{} reqs, {:.1} rps, {:.1}ms avg",
            total_requests, rps, mean_latency
        );
        let details = vec![
            format!("Total Requests: {}", total_requests),
            format!("Successful: {}", successful),
            format!("Failed: {}", failed),
            format!("Requests/sec: {:.2}", rps),
            format!("Mean Latency: {:.2}ms", mean_latency),
        ];
        self.add_entry("LoadTest".to_string(), target.to_string(), summary, details);
    }

    pub fn add_port_scan_result(
        &mut self,
        target: &str,
        ports_scanned: usize,
        open_ports: Vec<u16>,
    ) {
        let summary = format!("{} ports scanned, {} open", ports_scanned, open_ports.len());
        let details = open_ports
            .iter()
            .map(|p| format!("Port {} open", p))
            .collect();
        self.add_entry("PortScan".to_string(), target.to_string(), summary, details);
    }

    pub fn add_endpoint_scan_result(
        &mut self,
        target: &str,
        endpoints_found: usize,
        interesting: usize,
    ) {
        let summary = format!("{} endpoints, {} interesting", endpoints_found, interesting);
        let details = vec![
            format!("Endpoints found: {}", endpoints_found),
            format!("Interesting findings: {}", interesting),
        ];
        self.add_entry(
            "EndpointScan".to_string(),
            target.to_string(),
            summary,
            details,
        );
    }

    pub fn add_fingerprint_result(
        &mut self,
        target: &str,
        services_identified: usize,
        services: Vec<String>,
    ) {
        let summary = format!("{} services identified", services_identified);
        let details = services;
        self.add_entry(
            "Fingerprint".to_string(),
            target.to_string(),
            summary,
            details,
        );
    }

    pub fn add_waf_result(
        &mut self,
        target: &str,
        waf_detected: bool,
        waf_name: &str,
        bypasses_successful: usize,
    ) {
        let summary = if waf_detected {
            format!("WAF: {}, {} bypasses worked", waf_name, bypasses_successful)
        } else {
            "No WAF detected".to_string()
        };
        let details = vec![
            format!("WAF Detected: {}", waf_detected),
            if waf_detected {
                format!("WAF Name: {}", waf_name)
            } else {
                "No WAF found".to_string()
            },
            format!("Successful bypasses: {}", bypasses_successful),
        ];
        self.add_entry("WAF".to_string(), target.to_string(), summary, details);
    }

    pub fn add_pipeline_result(
        &mut self,
        target: &str,
        stages_completed: usize,
        total_stages: usize,
        duration_ms: u64,
    ) {
        let summary = format!(
            "{}/{} stages, {:.1}s total",
            stages_completed,
            total_stages,
            duration_ms as f64 / 1000.0
        );
        let details = vec![
            format!("Stages completed: {}/{}", stages_completed, total_stages),
            format!("Total duration: {:.1}s", duration_ms as f64 / 1000.0),
        ];
        self.add_entry("Pipeline".to_string(), target.to_string(), summary, details);
    }

    pub fn add_recon_result(&mut self, target: &str, domain: String, ip_address: String) {
        let summary = format!("Domain: {}, IP: {}", domain, ip_address);
        let details = vec![
            format!("Target: {}", target),
            format!("Domain: {}", domain),
            format!("IP Address: {}", ip_address),
        ];
        self.add_entry("Recon".to_string(), target.to_string(), summary, details);
    }
}
