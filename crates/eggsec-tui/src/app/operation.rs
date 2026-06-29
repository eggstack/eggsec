use super::App;
use crate::app::task_management::TaskBuilder;
use crate::tabs::{Tab, TabInput};
#[allow(unused_imports)]
use eggsec::config::OperationRisk;
use eggsec::config::{OperationDescriptor, OperationMode};

impl App {
    pub(crate) fn is_direct_launch_tab(&self, tab: Tab) -> bool {
        tab.is_direct_launch()
    }

    /// Build an OperationDescriptor for the current tab/action that is compatible with the
    /// shared enforcement evaluator (same risk/capability/operation strings used by CLI handlers).
    /// Returns None for tabs/operations that have no target-bearing networked action.
    #[allow(unused_mut)]
    pub fn build_current_operation_descriptor(&self) -> Option<OperationDescriptor> {
        let tab = self.current_tab;
        let spec = crate::tabs::spec_for(tab).filter(|s| s.operation.is_some())?;
        let target = self.current_tab_target();
        let op_id = spec.operation.unwrap();

        if let Some(metadata) = eggsec::config::operation_metadata(op_id) {
            let mut descriptor = metadata.descriptor_for_target(
                if target.as_deref().unwrap_or("").is_empty() {
                    None
                } else {
                    target
                },
            );

            // Tab-specific overrides for runtime details that metadata cannot know
            // (dry-run mode, advanced mode, etc.)

            #[cfg(feature = "wireless-advanced")]
            {
                if self.current_tab == Tab::Wireless && self.tabs.wireless.active_mode {
                    if let Some((
                        _interface,
                        attack_type,
                        _bssid,
                        _client,
                        _frame_count,
                        _rate_limit,
                        dry_run,
                    )) = self.tabs.wireless.active_attack_config()
                    {
                        let risk = if dry_run {
                            OperationRisk::SafeActive
                        } else {
                            OperationRisk::Intrusive
                        };
                        descriptor.operation = format!("wireless-{attack_type}");
                        descriptor.mode = OperationMode::DefenseLab;
                        descriptor.risk = risk;
                        descriptor.required_features = vec!["wireless-advanced".to_string()];
                        return Some(descriptor);
                    }
                }
            }

            #[cfg(feature = "db-pentest")]
            {
                if self.current_tab == Tab::DbPentest {
                    let is_advanced = self.tabs.db_pentest.advanced;
                    let dry = self.tabs.db_pentest.dry_run;
                    let risk = if is_advanced && !dry {
                        OperationRisk::Intrusive
                    } else {
                        OperationRisk::SafeActive
                    };
                    descriptor.mode = OperationMode::DefenseLab;
                    descriptor.risk = risk;
                    return Some(descriptor);
                }
            }

            Some(descriptor)
        } else {
            // Fallback for tabs without metadata entries
            let risk = crate::tabs::risk_from_group(spec.risk_group);
            let op = op_id.to_string();
            let required_features: Vec<String> = spec
                .feature
                .map(|f| vec![f.to_string()])
                .unwrap_or_default();
            Some(OperationDescriptor {
                operation: op,
                mode: OperationMode::StandardAssessment,
                risk,
                intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
                target: if target.as_deref().unwrap_or("").is_empty() {
                    None
                } else {
                    target
                },
                required_features,
                required_policy_flags: Vec::new(),
                requires_private_or_local_target: false,
                requires_explicit_scope: false,
                required_capabilities: Vec::new(),
            })
        }
    }

    /// Best-effort extraction of the primary target string from the current tab (for descriptor).
    pub(crate) fn current_tab_target(&self) -> Option<String> {
        match self.current_tab {
            Tab::Recon => self.tabs.recon.primary_target(),
            Tab::ScanPorts => self.tabs.scan_ports.primary_target(),
            Tab::ScanEndpoints => self.tabs.scan_endpoints.primary_target(),
            Tab::Fingerprint => self.tabs.fingerprint.primary_target(),
            Tab::Fuzz => self.tabs.fuzz.primary_target(),
            Tab::Waf => self.tabs.waf.primary_target(),
            Tab::WafStress => self.tabs.waf_stress.primary_target(),
            Tab::Scan => self.tabs.scan.primary_target(),
            Tab::Load => self.tabs.load.primary_target(),
            Tab::Stress => self.tabs.stress.primary_target(),
            Tab::Packet => self.tabs.packet.primary_target(),
            Tab::GraphQl => self.tabs.graphql.primary_target(),
            Tab::OAuth => self.tabs.oauth.primary_target(),
            Tab::Auth => self.tabs.auth.primary_target(),
            #[cfg(feature = "c2")]
            Tab::C2 => self.tabs.c2.primary_target(),
            #[cfg(feature = "nse")]
            Tab::Nse => self.tabs.nse.primary_target(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.tabs.hunt.primary_target(),
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.tabs.browser.primary_target(),
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.tabs.compliance.primary_target(),
            #[cfg(feature = "wireless")]
            Tab::Wireless => self.tabs.wireless.primary_target(),
            #[cfg(feature = "db-pentest")]
            Tab::DbPentest => self.tabs.db_pentest.primary_target(),
            #[cfg(feature = "web-proxy")]
            Tab::Intercept => self.tabs.intercept.primary_target(),
            _ => None,
        }
    }

    /// Produce a safe, minimal CLI equivalent for the current tab state.
    /// Returns None for non-executable tabs (Settings, History, Dashboard, Report, etc.).
    /// Never emits broad bypass flags (--yes, --allow-*, --insecure-tls, etc.).
    pub fn copy_cli_equivalent(&self) -> Option<String> {
        use crate::utils::shell_escape;
        let tab = self.current_tab;
        let cmd = tab.cli_command();
        if cmd == "unknown"
            || cmd == "Settings"
            || cmd == "History"
            || cmd == "Dashboard"
            || cmd == "eggsec report"
            || tab == Tab::Report
        {
            return None;
        }
        if !cmd.starts_with("eggsec ") {
            return None;
        }

        let target = self.current_tab_target().unwrap_or_default();
        let target_esc = if target.is_empty() {
            "''".to_string()
        } else {
            shell_escape(&target)
        };

        let mut out = format!("{} {}", cmd, target_esc);

        match tab {
            Tab::Recon => {
                let conc = self.tabs.recon.concurrency();
                if conc != 20 {
                    out.push_str(&format!(" --concurrency {}", conc));
                }
            }
            Tab::ScanPorts => {
                let ports = self.tabs.scan_ports.ports();
                if ports != "1-1024" {
                    out.push_str(&format!(" --ports {}", shell_escape(ports)));
                }
            }
            Tab::Fuzz => {
                let mp = self.tabs.fuzz.max_payloads();
                if mp > 0 {
                    out.push_str(&format!(" --max-payloads {}", mp));
                }
            }
            Tab::Auth => {
                if let Some(username) = self.tabs.auth.username() {
                    out.push_str(&format!(" --username {}", shell_escape(username)));
                }
                if let Some(passwords) = self.tabs.auth.password_list() {
                    out.push_str(&format!(" --wordlist {}", shell_escape(passwords)));
                }
            }
            #[cfg(feature = "c2")]
            Tab::C2 => {
                if let Some(campaign) = self.tabs.c2.campaign() {
                    out.push_str(&format!(" --campaign {}", shell_escape(campaign)));
                }
                out.push_str(" --dry-run");
            }
            #[cfg(feature = "wireless-advanced")]
            Tab::Wireless if self.tabs.wireless.active_mode => {
                if let Some((_, _, bssid, client, frame_count, rate_limit, dry_run)) =
                    self.tabs.wireless.active_attack_config()
                {
                    out.push_str(" deauth");
                    if let Some(bssid) = bssid {
                        out.push_str(&format!(" --bssid {}", shell_escape(&bssid)));
                    }
                    if let Some(client) = client {
                        out.push_str(&format!(" --client {}", shell_escape(&client)));
                    }
                    if frame_count != 100 {
                        out.push_str(&format!(" --count {}", frame_count));
                    }
                    if rate_limit != 10 {
                        out.push_str(&format!(" --fps {}", rate_limit));
                    }
                    if dry_run {
                        out.push_str(" --dry-run");
                    }
                }
            }
            _ => {}
        }

        if self.export_format != eggsec::types::OutputFormat::Pretty {
            let fmt = match self.export_format {
                eggsec::types::OutputFormat::Json => "json",
                eggsec::types::OutputFormat::Compact => "compact",
                eggsec::types::OutputFormat::Csv => "csv",
                eggsec::types::OutputFormat::Html => "html",
                eggsec::types::OutputFormat::Markdown => "markdown",
                eggsec::types::OutputFormat::Sarif => "sarif",
                eggsec::types::OutputFormat::Junit => "junit",
                _ => "pretty",
            };
            if fmt != "pretty" {
                out.push_str(&format!(" --format {}", fmt));
            }
        }

        if let Some(ref p) = self.enforcement_state.loaded_scope.path {
            if self.enforcement_state.loaded_scope.source
                == eggsec::config::ScopeSource::CliScopeFile
                || self.enforcement_state.loaded_scope.source
                    == eggsec::config::ScopeSource::ConfigFile
            {
                out.push_str(&format!(" --scope {}", shell_escape(p)));
            }
        }

        Some(out)
    }

    pub(crate) fn build_current_task(&self) -> Option<crate::workers::TaskConfig> {
        match self.current_tab {
            Tab::Recon => self.tabs.recon.build_task_config(),
            Tab::Load => self.tabs.load.build_task_config(),
            Tab::ScanPorts => self.tabs.scan_ports.build_task_config(),
            Tab::ScanEndpoints => self.tabs.scan_endpoints.build_task_config(),
            Tab::Fingerprint => self.tabs.fingerprint.build_task_config(),
            Tab::Fuzz => self.tabs.fuzz.build_task_config(),
            Tab::Waf => self.tabs.waf.build_task_config(),
            Tab::WafStress => self.tabs.waf_stress.build_task_config(),
            Tab::Scan => self.tabs.scan.build_task_config(),
            Tab::Packet => self.tabs.packet.build_task_config(),
            Tab::GraphQl => self.tabs.graphql.build_task_config(),
            Tab::OAuth => self.tabs.oauth.build_task_config(),
            Tab::Auth => self.tabs.auth.build_task_config(),
            #[cfg(feature = "c2")]
            Tab::C2 => self.tabs.c2.build_task_config(),
            Tab::Cluster => self.tabs.cluster.build_task_config(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.tabs.hunt.build_task_config(),
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.tabs.browser.build_task_config(),
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.tabs.compliance.build_task_config(),
            #[cfg(feature = "database")]
            Tab::Storage => self.tabs.storage.build_task_config(),
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.tabs.integrations.build_task_config(),
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.tabs.workflow.build_task_config(),
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.tabs.vuln.build_task_config(),
            #[cfg(feature = "wireless")]
            Tab::Wireless => self.tabs.wireless.build_task_config(),
            #[cfg(feature = "db-pentest")]
            Tab::DbPentest => self.tabs.db_pentest.build_task_config(),
            #[cfg(feature = "web-proxy")]
            Tab::Intercept => self.tabs.intercept.build_task_config(),
            _ => None,
        }
    }
}
