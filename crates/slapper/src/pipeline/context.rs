use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::scanner::endpoints::EndpointResult;
use crate::scanner::fingerprint::ServiceFingerprint;
use crate::scanner::ports::PortResult;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineContext {
    pub target: String,
    pub open_ports: Vec<u16>,
    pub services: FxHashMap<u16, ServiceFingerprint>,
    pub endpoints: Vec<EndpointResult>,
    pub port_results: Vec<PortResult>,
    pub http_ports: Vec<u16>,
}

impl PipelineContext {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }

    pub fn should_run_fuzzer(&self) -> bool {
        self.endpoints.iter().any(|e| e.interesting)
    }

    pub fn get_http_ports(&self) -> Vec<u16> {
        self.services
            .iter()
            .filter(|(_, s)| s.service == "HTTP" || s.service == "HTTPS")
            .map(|(p, _)| *p)
            .collect()
    }

    pub fn get_base_url(&self) -> Option<String> {
        if self.http_ports.contains(&443) {
            Some(format!("https://{}", self.target))
        } else if self.http_ports.contains(&80) {
            Some(format!("http://{}", self.target))
        } else if !self.http_ports.is_empty() {
            Some(format!("http://{}:{}", self.target, self.http_ports[0]))
        } else {
            None
        }
    }

    pub fn update_ports(&mut self, ports: Vec<PortResult>) {
        self.open_ports = ports.iter().map(|p| p.port).collect();
        self.port_results = ports;
    }

    pub fn update_services(&mut self, services: Vec<ServiceFingerprint>) {
        for service in services {
            self.services.insert(service.port, service);
        }
        self.http_ports = self.get_http_ports();
    }

    pub fn update_endpoints(&mut self, endpoints: Vec<EndpointResult>) {
        self.endpoints = endpoints;
    }
}
