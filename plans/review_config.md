# Config Module Architecture Review

**Document:** architecture/config.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 110

## Verified Claims
- [SlapperConfig in settings.rs]: Verified at `config/settings.rs:93`
- [HttpConfig with retry_delay_ms]: Verified at `config/http.rs:27`
- [ScanConfig with port_timeout_secs]: Verified at `config/scan.rs:37`
- [AlertChannelsConfig.channels uses FxHashMap]: Verified at `config/settings.rs:22`
- [WebhookConfigEntry.headers uses FxHashMap]: Verified at `config/settings.rs:39`
- [HttpConfig.default_headers uses FxHashMap]: Verified at `config/http.rs:39`
- [SlapperConfig.profiles uses FxHashMap]: Verified at `config/settings.rs:107`
- [WebhookConfig.headers uses FxHashMap]: Verified at `config/scan.rs:132`
- [Scope::is_target_allowed returns Result<bool, ScopeError>]: Verified at `config/scope.rs:100`
- [Scope::validate_url returns Result<bool, ScopeError>]: Verified at `config/scope.rs:188`
- [Scope::is_port_allowed returns bool]: Verified at `config/scope.rs:170`
- [ScopeRule::new(pattern)]: Verified at `config/scope.rs:213`
- [ScopeRule::with_cidr(cidr) via IpNetwork::from_str()]: Verified at `config/scope.rs:221-230`
- [ScopeError enum at scope.rs:400-422 has 7 variants]: Verified at `config/scope.rs:400-422`
- [TOML/YAML Loading in Loader]: Verified at `config/loader.rs:40-50`
- [ConfigError enum with 4 variants]: Verified at `config/settings.rs:697-710`

## Discrepancies
- [Scope::validate() max_requests_per_second check]: Documented as "must be greater than 0", but actual implementation is `rate > 10000` check (lines 62-72 in scope.rs). The validation is "must be in range 1..=10000 if set". Minor wording difference but the semantic is correct.
- [TUI Settings Tab description]: Documented that settings are in `tui/tabs/settings/main.rs` - UNVERIFIED (file not read during this review, but document notes this was previously reviewed)

## Bugs Found
- None found. The configuration system is well-implemented.

## Improvement Opportunities
- [Documentation precision]: The `Scope::validate()` description could be more precise about the 1..=10000 range check for `max_requests_per_second` (medium priority)

## Stale Items
- None identified

## Code Interrogation Findings
- [Security observation]: Private IP blocking is correctly implemented in `is_private_ip()` at `scope.rs:382-398`. The function checks:
  - IPv4: 10.x.x.x, 172.16-31.x.x, 192.168.x.x, 169.254.x.x (link-local), 127.x.x.x
  - IPv6: loopback, ULA (fc00::/7), link-local (fe80::/10)
- [Observation]: The `TargetScope::parse()` function at line 273-320 has a comment stating "Private IP check is deferred to scope rule evaluation in is_target_allowed()" - this appears intentional based on the architecture notes about scope rule evaluation happening AFTER private IP check.