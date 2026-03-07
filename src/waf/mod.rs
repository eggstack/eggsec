#![allow(dead_code)]

pub mod bypass;
pub mod detector;
pub mod payloads;
pub mod waf_patterns;
pub mod types;

use anyhow::Result;
use std::io::Write;
use std::time::Instant;

use crate::cli::WafArgs;
use crate::config::SlapperConfig;

pub use detector::{WafDetector, WafDetectionResult};
pub use bypass::{BypassEngine, BypassResult, get_profile_by_name, get_auto_profile, WafProfile, TestType};
pub use types::{Finding, OwaspCategory, ScanResults, Severity};

pub async fn run_cli(args: WafArgs, _config: &SlapperConfig) -> Result<()> {
    let mut engine = WafEngine::new(args)?;
    engine.run().await
}

pub struct WafEngine {
    args: WafArgs,
    detector: WafDetector,
    bypass_engine: Option<BypassEngine>,
    selected_profile: Option<String>,
}

impl WafEngine {
    pub fn new(args: WafArgs) -> Result<Self> {
        let detector = WafDetector::new()?;
        Ok(Self {
            args,
            detector,
            bypass_engine: None,
            selected_profile: None,
        })
    }

    fn select_profile(&mut self, detection: &WafDetectionResult) -> Option<WafProfile> {
        let profile_name = &self.args.profile;
        if profile_name.to_lowercase() == "auto" {
            if let Some(ref waf_name) = detection.waf_name {
                let waf_lower = waf_name.to_lowercase();
                
                if let Some(profile) = get_profile_by_name(&waf_lower) {
                    self.selected_profile = Some(profile.name.clone());
                    return Some(profile);
                }
                
                for profile in bypass::get_waf_profiles() {
                    for sig in &profile.detection_signatures {
                        let sig_lower = sig.to_lowercase();
                        if waf_lower == sig_lower || 
                           waf_lower.starts_with(&sig_lower) ||
                           waf_lower.ends_with(&sig_lower) ||
                           waf_lower.contains(&format!(" {}", &sig_lower)) ||
                           waf_lower.contains(&format!("{}", &sig_lower))
                        {
                            self.selected_profile = Some(profile.name.clone());
                            return Some(profile);
                        }
                    }
                }
                eprintln!("[WARN] Auto-detected WAF '{}' but no matching profile found, using generic profile", waf_name);
            }
            let profile = get_auto_profile();
            self.selected_profile = Some(profile.name.clone());
            return Some(profile);
        } else if let Some(profile) = get_profile_by_name(profile_name) {
            self.selected_profile = Some(profile.name.clone());
            return Some(profile);
        }
        
        eprintln!("[WARN] Unknown profile '{}', falling back to auto profile", profile_name);
        let profile = get_auto_profile();
        self.selected_profile = Some(profile.name.clone());
        Some(profile)
    }

    pub async fn run(&mut self) -> Result<()> {
        let start = Instant::now();
        
        if self.args.verbose {
            eprintln!("Detecting WAF on {}", self.args.url);
        }
        
        let detection = self.detector.detect(&self.args.url).await?;
        
        if !self.args.bypass && !self.args.header_bypass && !self.args.smuggling && !self.args.evasion {
            self.print_detection(&detection);
            
            if self.args.verbose {
                if detection.waf_name.is_some() {
                    eprintln!("WAF detected: {}", detection.waf_name.as_ref().unwrap());
                } else {
                    eprintln!("No WAF detected");
                }
            }
            return Ok(());
        }

        let profile = self.select_profile(&detection);
        let test_type = self.args.test_type.as_ref()
            .map(|t| TestType::from_string(t))
            .unwrap_or(TestType::All);
        self.bypass_engine = Some(BypassEngine::new(&self.args, profile, test_type)?);
        
        if self.args.verbose {
            eprintln!("Attempting WAF bypasses...");
        }
        
        let bypass_results = match &self.bypass_engine {
            Some(engine) => engine.run_bypasses(&detection).await?,
            None => {
                eprintln!("[ERROR] Failed to initialize bypass engine");
                return Ok(());
            }
        };
        
        let findings = bypass_results.iter().map(|br| {
            let severity = if br.success { Severity::High } else { Severity::Low };
            let owasp = if br.success { 
                OwaspCategory::A05_2021_SecurityMisconfiguration 
            } else { 
                OwaspCategory::A05_2021_SecurityMisconfiguration 
            };
            
            Finding::new(
                format!("WAF Bypass - {:?}", br.technique),
                br.description.clone(),
                severity,
                owasp,
                detection.waf_name.clone(),
                br.success,
                format!("{:?}", br.technique),
                br.description.clone(),
                br.status_code,
            )
        }).collect::<Vec<_>>();

        let scan_results = ScanResults::new(
            self.args.url.clone(),
            start.elapsed().as_millis() as u64,
            Some(detection.clone()),
            findings,
        );

        if self.args.verbose {
            let successful = bypass_results.iter().filter(|r| r.success).count();
            let total = bypass_results.len();
            eprintln!("WAF bypass complete: {} successful out of {} attempts", successful, total);
        }

        let output = if self.args.json {
            serde_json::to_string_pretty(&scan_results)?
        } else {
            String::new()
        };

        if let Some(ref output_file) = self.args.output {
            std::fs::write(output_file, &output)?;
            if self.args.verbose {
                eprintln!("Results written to {}", output_file);
            }
        } else {
            self.print_results(&detection, &bypass_results);
            if self.args.json {
                println!("{}", output);
            }
        }
        
        Ok(())
    }

    fn print_detection(&self, detection: &WafDetectionResult) {
        if self.args.json {
            println!("{}", serde_json::to_string_pretty(&detection).unwrap());
            return;
        }

        writeln!(std::io::stdout(), "WAF Detection Results").unwrap();
        
        if let Some(ref waf_name) = detection.waf_name {
            println!("waf: {} ({}% confidence)", waf_name, detection.confidence);
            if !detection.matched_headers.is_empty() {
                println!("matched headers: {}", detection.matched_headers.join(", "));
            }
            if !detection.matched_cookies.is_empty() {
                println!("matched cookies: {}", detection.matched_cookies.join(", "));
            }
        } else {
            println!("waf: none detected");
        }
        
        if let Some(ref profile_name) = self.selected_profile {
            println!("profile: {}", profile_name);
        }
    }

    fn print_results(&self, detection: &WafDetectionResult, bypass_results: &[BypassResult]) {
        if self.args.json {
            let output = serde_json::json!({
                "detection": detection,
                "bypass_results": bypass_results,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            return;
        }

        self.print_detection(detection);
        
        println!();
        
        let successful: Vec<_> = bypass_results.iter().filter(|r| r.success).collect();
        let failed: Vec<_> = bypass_results.iter().filter(|r| !r.success).collect();

        for result in &successful {
            println!("[+] {:?}: {}", result.technique, result.description);
        }
        
        for result in &failed {
            println!("[-] {:?}: {}", result.technique, result.description);
        }

        println!("\nbypasses: {} / {} successful", successful.len(), bypass_results.len());
    }
}
