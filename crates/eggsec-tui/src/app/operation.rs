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

        if let Some(metadata) = eggsec::config::metadata_for_tool_id(op_id) {
            let mut descriptor =
                metadata.descriptor_for_target(if target.as_deref().unwrap_or("").is_empty() {
                    None
                } else {
                    target
                });

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
            Some(OperationDescriptor::new(
                op,
                OperationMode::StandardAssessment,
                risk,
                vec![eggsec::config::IntendedUse::WebAssessment],
                if target.as_deref().unwrap_or("").is_empty() {
                    None
                } else {
                    target
                },
                required_features,
                Vec::new(),
                false,
                false,
                Vec::new(),
            ))
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

    /// Phase 2.7: stable argument-vector representation for CLI equivalents.
    ///
    /// Quoting happens only at the final clipboard/string boundary
    /// (`copy_cli_equivalent`); tests parse this argv through the real Clap
    /// `Cli` parser and convert back to the canonical request for semantic
    /// equality. Returns `None` for UI-only state (explicit unsupported result
    /// rather than a misleading command).
    pub fn cli_argv(&self) -> Option<Vec<String>> {
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
        let sub = cmd.strip_prefix("eggsec ").unwrap_or("");
        if sub.is_empty() {
            return None;
        }
        // Only `db pentest` is a multi-word subcommand backing a tab.
        if sub.contains(' ') && tab != Tab::DbPentest {
            return None;
        }
        let mut argv: Vec<String> = vec!["eggsec".to_string()];
        // `db pentest` expands to two argv elements.
        for part in sub.split_whitespace() {
            argv.push(part.to_string());
        }
        let target = self.current_tab_target().unwrap_or_default();
        argv.push(if target.is_empty() {
            String::new()
        } else {
            target
        });

        match tab {
            Tab::Recon => {
                let conc = self.tabs.recon.concurrency();
                if conc != 20 {
                    argv.push("--concurrency".to_string());
                    argv.push(conc.to_string());
                }
            }
            Tab::ScanPorts => {
                let ports = self.tabs.scan_ports.ports();
                if ports != "1-1024" {
                    argv.push("--ports".to_string());
                    argv.push(ports.to_string());
                }
            }
            Tab::Fuzz => {
                // Phase 2.7: `Max Payloads` is TUI-only (no CLI flag); omit it
                // rather than emitting a misleading command. `--concurrency`
                // is the real CLI equivalent (both default to 10).
                let conc = self.tabs.fuzz.concurrency();
                if conc != 10 {
                    argv.push("--concurrency".to_string());
                    argv.push(conc.to_string());
                }
            }
            Tab::Auth => {
                if let Some(username) = self.tabs.auth.username() {
                    argv.push("--username".to_string());
                    argv.push(username.to_string());
                }
                if let Some(passwords) = self.tabs.auth.password_list() {
                    argv.push("--wordlist".to_string());
                    argv.push(passwords.to_string());
                }
            }
            #[cfg(feature = "c2")]
            Tab::C2 => {
                if let Some(campaign) = self.tabs.c2.campaign() {
                    argv.push("--campaign".to_string());
                    argv.push(campaign.to_string());
                }
                argv.push("--dry-run".to_string());
            }
            #[cfg(feature = "wireless-advanced")]
            Tab::Wireless if self.tabs.wireless.active_mode => {
                if let Some((_, _, bssid, client, frame_count, rate_limit, dry_run)) =
                    self.tabs.wireless.active_attack_config()
                {
                    argv.push("deauth".to_string());
                    if let Some(bssid) = bssid {
                        argv.push("--bssid".to_string());
                        argv.push(bssid);
                    }
                    if let Some(client) = client {
                        argv.push("--client".to_string());
                        argv.push(client);
                    }
                    if frame_count != 100 {
                        argv.push("--count".to_string());
                        argv.push(frame_count.to_string());
                    }
                    if rate_limit != 10 {
                        argv.push("--fps".to_string());
                        argv.push(rate_limit.to_string());
                    }
                    if dry_run {
                        argv.push("--dry-run".to_string());
                    }
                }
            }
            _ => {}
        }

        // Phase 2.7: map the TUI export format to real CLI flags.
        // Most commands expose `--json` (bool); only some (e.g. fuzz) expose
        // `--format`. Never emit a flag the real Clap tree rejects.
        if self.export_format != eggsec::types::OutputFormat::Pretty {
            match self.export_format {
                eggsec::types::OutputFormat::Json => {
                    // `--json` exists on recon/scan-ports/fuzz and most tabs.
                    argv.push("--json".to_string());
                }
                eggsec::types::OutputFormat::Compact
                | eggsec::types::OutputFormat::Csv
                | eggsec::types::OutputFormat::Html
                | eggsec::types::OutputFormat::Markdown
                | eggsec::types::OutputFormat::Sarif
                | eggsec::types::OutputFormat::Junit => {
                    // Only emit `--format` for commands that accept it
                    // (currently Fuzz in the round-trip set). Others omit:
                    // a misleading `--format` would fail Clap parsing.
                    if matches!(tab, Tab::Fuzz) {
                        let fmt = match self.export_format {
                            eggsec::types::OutputFormat::Compact => "compact",
                            eggsec::types::OutputFormat::Csv => "csv",
                            eggsec::types::OutputFormat::Html => "html",
                            eggsec::types::OutputFormat::Markdown => "markdown",
                            eggsec::types::OutputFormat::Sarif => "sarif",
                            eggsec::types::OutputFormat::Junit => "junit",
                            _ => "pretty",
                        };
                        argv.push("--format".to_string());
                        argv.push(fmt.to_string());
                    }
                }
                _ => {}
            }
        }

        if let Some(ref p) = self.enforcement_state.state().loaded_scope.path {
            if self.enforcement_state.state().loaded_scope.source
                == eggsec::config::ScopeSource::CliScopeFile
                || self.enforcement_state.state().loaded_scope.source
                    == eggsec::config::ScopeSource::ConfigFile
            {
                argv.push("--scope".to_string());
                argv.push(p.clone());
            }
        }

        Some(argv)
    }

    /// Produce a safe, minimal CLI equivalent for the current tab state.
    /// Returns None for non-executable tabs (Settings, History, Dashboard, Report, etc.).
    /// Never emits broad bypass flags (--yes, --allow-*, --insecure-tls, etc.).
    pub fn copy_cli_equivalent(&self) -> Option<String> {
        use crate::utils::shell_escape;
        let argv = self.cli_argv()?;
        // Quote only at the final clipboard/string boundary (Phase 2.7).
        let mut parts: Vec<String> = Vec::with_capacity(argv.len());
        for (i, arg) in argv.iter().enumerate() {
            if i == 0 {
                parts.push(arg.clone());
                continue;
            }
            if arg.is_empty() {
                parts.push("''".to_string());
            } else {
                parts.push(shell_escape(arg));
            }
        }
        Some(parts.join(" "))
    }

    pub(crate) fn build_current_task(&self) -> Option<eggsec_runtime::RunRequest> {
        match self.current_tab {
            Tab::Recon => self.tabs.recon.build_run_request(),
            Tab::Load => self.tabs.load.build_run_request(),
            Tab::ScanPorts => self.tabs.scan_ports.build_run_request(),
            Tab::ScanEndpoints => self.tabs.scan_endpoints.build_run_request(),
            Tab::Fingerprint => self.tabs.fingerprint.build_run_request(),
            Tab::Fuzz => self.tabs.fuzz.build_run_request(),
            Tab::Waf => self.tabs.waf.build_run_request(),
            Tab::WafStress => self.tabs.waf_stress.build_run_request(),
            Tab::Scan => self.tabs.scan.build_run_request(),
            Tab::Packet => self.tabs.packet.build_run_request(),
            Tab::GraphQl => self.tabs.graphql.build_run_request(),
            Tab::OAuth => self.tabs.oauth.build_run_request(),
            Tab::Auth => self.tabs.auth.build_run_request(),
            #[cfg(feature = "c2")]
            Tab::C2 => self.tabs.c2.build_run_request(),
            Tab::Cluster => self.tabs.cluster.build_run_request(),
            #[cfg(feature = "advanced-hunting")]
            Tab::Hunt => self.tabs.hunt.build_run_request(),
            #[cfg(feature = "headless-browser")]
            Tab::Browser => self.tabs.browser.build_run_request(),
            #[cfg(feature = "compliance")]
            Tab::Compliance => self.tabs.compliance.build_run_request(),
            #[cfg(feature = "database")]
            Tab::Storage => self.tabs.storage.build_run_request(),
            #[cfg(feature = "external-integrations")]
            Tab::Integrations => self.tabs.integrations.build_run_request(),
            #[cfg(feature = "finding-workflow")]
            Tab::Workflow => self.tabs.workflow.build_run_request(),
            #[cfg(feature = "vuln-management")]
            Tab::Vuln => self.tabs.vuln.build_run_request(),
            #[cfg(feature = "wireless")]
            Tab::Wireless => self.tabs.wireless.build_run_request(),
            #[cfg(feature = "db-pentest")]
            Tab::DbPentest => self.tabs.db_pentest.build_run_request(),
            #[cfg(feature = "web-proxy")]
            Tab::Intercept => self.tabs.intercept.build_run_request(),
            _ => None,
        }
    }
}
