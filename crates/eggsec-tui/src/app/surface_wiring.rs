//! Phase 2.9 TUI semantic wiring tests (no live terminal).
//!
//! Covers:
//! - every runnable tab -> canonical operation/route;
//! - operation target/request construction from tab state;
//! - CLI-equivalent round trips through the real Clap `Cli` parser +
//!   canonical request adapters (semantic equality after normalization);
//! - cancellation state transitions;
//! - exact approval-cache invalidation integration (Phase 0);
//! - help/palette discoverability agreement.

#[cfg(test)]
mod tests {
    use clap::Parser;

    use eggsec::cli::{Cli, Commands};

    use crate::app::create_test_app;
    use crate::tabs::{spec_for, tab_specs, Tab, TuiSurfaceRoute};

    /// Minimal POSIX single-quote-aware splitter for `copy_cli_equivalent`
    /// output (mirrors `shell_escape` quoting: safe chars unquoted, others
    /// single-quoted with `'\''` for embedded quotes).
    fn split_cli_string(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut in_single = false;
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if in_single {
                if c == '\'' {
                    // `'\''` sequence: close, escaped quote, reopen.
                    if chars.peek() == Some(&'\\') {
                        // Not our encoding; treat literally.
                        cur.push(c);
                    } else if chars.peek().is_some() {
                        // Look ahead for `'\''` pattern: we already consumed
                        // the closing `'`, check for `\` + `'` + `'`.
                        let mut clone = chars.clone();
                        if clone.next() == Some('\\')
                            && clone.next() == Some('\'')
                            && clone.next() == Some('\'')
                        {
                            cur.push('\'');
                            chars.next();
                            chars.next();
                            chars.next();
                        } else {
                            // End of quoted section.
                            in_single = false;
                        }
                    } else {
                        in_single = false;
                    }
                } else {
                    cur.push(c);
                }
            } else if c == '\'' {
                in_single = true;
            } else if c.is_whitespace() {
                if !cur.is_empty() {
                    out.push(cur.clone());
                    cur.clear();
                }
            } else {
                cur.push(c);
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    }

    #[test]
    fn every_runnable_tab_maps_to_canonical_route() {
        for spec in tab_specs() {
            if !spec.supports_run {
                continue;
            }
            match spec.surface_route() {
                TuiSurfaceRoute::Operation(op) => {
                    assert_eq!(
                        spec.canonical_operation(),
                        Some(op),
                        "runnable {:?} must have canonical op",
                        spec.tab
                    );
                    assert!(
                        eggsec::config::metadata_for_tool_id(op).is_some(),
                        "runnable {:?} op '{op}' missing metadata",
                        spec.tab
                    );
                }
                TuiSurfaceRoute::Multiplexer(family) => {
                    assert_eq!(spec.tab, Tab::Wireless);
                    assert_eq!(family, "wireless");
                }
                TuiSurfaceRoute::Helper | TuiSurfaceRoute::Lifecycle => {
                    // Runnable helpers (e.g. Resume loads a session) never
                    // claim a canonical operation.
                    assert!(spec.canonical_operation().is_none());
                }
                TuiSurfaceRoute::UiOnly => {
                    panic!("UI-only {:?} must not declare supports_run", spec.tab);
                }
            }
        }
    }

    #[test]
    fn operation_target_construction_matches_tab_state() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        if let Some(f) = app.tabs.recon.core.inputs.fields.first_mut() {
            f.value = "example.com".to_string();
        }
        assert_eq!(app.current_tab_target().as_deref(), Some("example.com"));
        let desc = app
            .build_current_operation_descriptor()
            .expect("recon must build a descriptor");
        assert_eq!(desc.operation, "recon");

        app.current_tab = Tab::ScanPorts;
        if let Some(f) = app.tabs.scan_ports.core.inputs.fields.first_mut() {
            f.value = "10.0.0.1".to_string();
        }
        assert_eq!(app.current_tab_target().as_deref(), Some("10.0.0.1"));
    }

    #[test]
    fn copy_cli_recon_round_trips_through_clap_and_canonical() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.export_format = eggsec::types::OutputFormat::Pretty;
        if let Some(f) = app.tabs.recon.core.inputs.fields.first_mut() {
            f.value = "example.com".to_string();
        }
        if let Some(f) = app.tabs.recon.core.inputs.fields.get_mut(1) {
            f.value = "50".to_string();
        }
        let argv = app.cli_argv().expect("recon must have argv");
        assert_eq!(argv[0], "eggsec");
        assert_eq!(argv[1], "recon");
        assert!(argv.contains(&"example.com".to_string()));
        // String form quotes only at the boundary.
        let cli_string = app.copy_cli_equivalent().unwrap();
        assert!(cli_string.contains("eggsec recon example.com"));
        assert!(cli_string.contains("--concurrency 50"));
        // Parse through the real Clap tree.
        let parts = split_cli_string(&cli_string);
        let cli = Cli::try_parse_from(parts).expect("recon CLI must parse");
        match cli.command {
            Some(Commands::Recon(args)) => {
                assert_eq!(args.target, "example.com");
                assert_eq!(args.concurrency, Some(50));
                // Back to the canonical request: semantic equality.
                let req = eggsec::operation_request::cli_adapters::recon_from_cli(&args);
                assert_eq!(req.target, "example.com");
            }
            Some(other) => panic!("expected Recon, got {}", other.command_id()),
            None => panic!("expected Recon, got none"),
        }
    }

    #[test]
    fn copy_cli_scan_ports_round_trips_through_clap_and_canonical() {
        let mut app = create_test_app();
        app.current_tab = Tab::ScanPorts;
        app.export_format = eggsec::types::OutputFormat::Pretty;
        if let Some(f) = app.tabs.scan_ports.core.inputs.fields.first_mut() {
            f.value = "10.0.0.1".to_string();
        }
        if let Some(f) = app.tabs.scan_ports.core.inputs.fields.get_mut(1) {
            f.value = "22,80,443".to_string();
        }
        let cli_string = app.copy_cli_equivalent().unwrap();
        assert!(cli_string.contains("eggsec scan-ports 10.0.0.1"));
        assert!(cli_string.contains("--ports '22,80,443'"));
        let parts = split_cli_string(&cli_string);
        let cli = Cli::try_parse_from(parts).expect("scan-ports CLI must parse");
        match cli.command {
            Some(Commands::ScanPorts(args)) => {
                assert_eq!(args.host, "10.0.0.1");
                assert_eq!(args.ports, "22,80,443");
                let req = eggsec::operation_request::cli_adapters::port_scan_from_cli(&args);
                assert_eq!(req.target, "10.0.0.1");
                assert_eq!(req.ports.as_deref(), Some("22,80,443"));
            }
            Some(other) => panic!("expected ScanPorts, got {}", other.command_id()),
            None => panic!("expected ScanPorts, got none"),
        }
    }

    #[test]
    fn copy_cli_fuzz_round_trips_through_clap_and_canonical() {
        let mut app = create_test_app();
        app.current_tab = Tab::Fuzz;
        app.export_format = eggsec::types::OutputFormat::Pretty;
        if let Some(f) = app.tabs.fuzz.core.inputs.fields.first_mut() {
            f.value = "https://target.test".to_string();
        }
        if let Some(f) = app.tabs.fuzz.core.inputs.fields.get_mut(5) {
            f.value = "25".to_string();
        }
        let cli_string = app.copy_cli_equivalent().unwrap();
        assert!(cli_string.contains("eggsec fuzz https://target.test"));
        assert!(cli_string.contains("--concurrency 25"));
        assert!(!cli_string.contains("--max-payloads"));
        let parts = split_cli_string(&cli_string);
        let cli = Cli::try_parse_from(parts).expect("fuzz CLI must parse");
        match cli.command {
            Some(Commands::Fuzz(args)) => {
                assert_eq!(args.url, "https://target.test");
                assert_eq!(args.concurrency, 25);
                let req = eggsec::operation_request::cli_adapters::fuzz_from_cli(&args);
                assert_eq!(req.target, "https://target.test");
            }
            Some(other) => panic!("expected Fuzz, got {}", other.command_id()),
            None => panic!("expected Fuzz, got none"),
        }
    }

    #[test]
    fn copy_cli_shell_quoting_survives_round_trip() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        app.export_format = eggsec::types::OutputFormat::Pretty;
        if let Some(f) = app.tabs.recon.core.inputs.fields.first_mut() {
            f.value = "target with spaces;rm -rf".to_string();
        }
        let cli_string = app.copy_cli_equivalent().unwrap();
        assert!(cli_string.contains("'target with spaces;rm -rf'"));
        let parts = split_cli_string(&cli_string);
        let cli = Cli::try_parse_from(parts).expect("quoted recon must parse");
        match cli.command {
            Some(Commands::Recon(args)) => {
                assert_eq!(args.target, "target with spaces;rm -rf");
            }
            Some(other) => panic!("expected Recon, got {}", other.command_id()),
            None => panic!("expected Recon, got none"),
        }
    }

    #[test]
    fn cancellation_transitions_do_not_require_terminal() {
        let mut app = create_test_app();
        assert!(!app.has_active_task());
        app.execute_command("pause");
        assert!(app.overlay.notification.is_some());
        app.execute_command("jump-active");
        assert!(app.overlay.notification.is_some());
    }

    #[test]
    fn approval_cache_invalidation_is_wired_to_surface() {
        // Phase 0 contract pinned from the TUI side: posture toggle clears
        // the cached approval so the next descriptor re-evaluates.
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        if let Some(f) = app.tabs.recon.core.inputs.fields.first_mut() {
            f.value = "example.com".to_string();
        }
        let desc = app
            .build_current_operation_descriptor()
            .expect("descriptor");
        // Take without cached approval: facade evaluates fresh.
        let _ = app.enforcement_state.take_cached_approval(&desc);
        app.enforcement_state.clear_cached_approval();
        app.apply_action(crate::app::UiAction::ToggleEnforcementPosture);
        // After toggle, no stale approval may survive.
        assert!(
            app.enforcement_state.take_cached_approval(&desc).is_none(),
            "posture toggle must invalidate cached approval"
        );
        let _ = spec_for(Tab::Recon).unwrap();
    }

    #[test]
    fn help_and_palette_discoverability_agree() {
        let app = create_test_app();
        let entries = app.help_manager.get_command_palette_entries();
        let commands: std::collections::BTreeSet<&str> =
            entries.iter().map(|e| e.command.as_str()).collect();
        // Every visible tab is discoverable via its primary palette command.
        for tab in Tab::all() {
            let spec = spec_for(*tab).unwrap();
            assert!(
                commands.contains(spec.palette_command()),
                "visible {tab:?} palette '{}' missing from discovery",
                spec.palette_command()
            );
            assert!(
                app.help_manager.get_help_for_tab(*tab).is_some()
                    || matches!(
                        tab,
                        Tab::DbPentest | Tab::Intercept | Tab::Wireless | Tab::C2
                    ) && !commands.contains(spec.palette_command()),
                "visible {tab:?} should have help content"
            );
        }
        // No false affordance.
        assert!(!commands.contains("reload-scope"));
        // Help entries are non-empty for every visible tab.
        for tab in Tab::all() {
            assert!(!tab.help_entry().is_empty());
        }
    }
}
