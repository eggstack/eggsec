# Policy Validation Results

Date: 2026-06-10
Branch/commit: main (working tree)

## Commands Run

- [x] cargo fmt --all
- [x] cargo test --lib -p eggsec (handler policy tests, policy unit tests, policy_decision tests)
- [x] cargo test -p eggsec-core
- [x] cargo test -p eggsec-output
- [x] cargo test -p eggsec-tool-core
- [x] cargo test --test policy_contract_tests -p eggsec
- [x] cargo clippy --lib -p eggsec
- [x] cargo build -p eggsec-cli
- [ ] cargo build -p eggsec-cli --features stress-testing (pre-existing failure on main)
- [ ] cargo build -p eggsec-cli --features packet-inspection (pre-existing failure on main)
- [ ] cargo build -p eggsec-cli --features "rest-api ai-integration" (pre-existing failure on main)
- [ ] cargo build -p eggsec-cli --features full (pre-existing failure on main)
- [ ] cargo test -p eggsec-nse (1 pre-existing test failure: sandbox_tests::test_sandbox_enabled_restricts_paths)

## Results

### Passed

| Command | Result |
|---------|--------|
| `cargo fmt --all` | PASS - all files formatted |
| `cargo test --lib -p eggsec -- commands::handlers::tests` | PASS - 18/18 tests passed |
| `cargo test --lib -p eggsec -- config::policy` | PASS - 12/12 tests passed |
| `cargo test --lib -p eggsec -- config::policy_decision` | PASS - 14/14 tests passed |
| `cargo test -p eggsec-core` | PASS - 23/23 tests passed |
| `cargo test -p eggsec-output` | PASS - 63/63 tests passed |
| `cargo test -p eggsec-tool-core` | PASS - 4/4 tests + 1 doc-test passed |
| `cargo test --test policy_contract_tests -p eggsec` | PASS - 9/9 tests passed |
| `cargo clippy --lib -p eggsec` | PASS - 18 pre-existing warnings, no new warnings |
| `cargo build -p eggsec-cli` | PASS - default features build succeeds |

### Pre-existing Failures (not caused by this change)

| Command | Result |
|---------|--------|
| `cargo build -p eggsec-cli --features stress-testing` | FAIL - type mismatch in `proxy/http_connect.rs` and `scanner/ports/spoofed.rs` (pre-existing) |
| `cargo build -p eggsec-cli --features packet-inspection` | FAIL - missing `AtomicUsize` import and type mismatch (pre-existing) |
| `cargo build -p eggsec-cli --features "rest-api ai-integration"` | FAIL - unresolved imports, type mismatches (pre-existing) |
| `cargo build -p eggsec-cli --features full` | FAIL - same as above (pre-existing) |
| `cargo test -p eggsec-nse` | FAIL - `test_sandbox_enabled_restricts_paths` assertion failure (pre-existing) |

### New Tests Added

18 regression tests in `crates/eggsec/src/commands/handlers/mod.rs`:

- `safe_active_allowed_by_default` - SafeActive operations pass policy
- `intrusive_denied_by_default` - Intrusive operations blocked without flag
- `intrusive_allowed_when_enabled` - Intrusive operations pass with flag
- `stress_test_denied_without_policy_flag` - StressTest blocked by default
- `stress_test_allowed_with_policy_flag` - StressTest passes with flag
- `raw_packet_denied_without_policy_flag` - RawPacket blocked by default
- `raw_packet_allowed_with_policy_flag` - RawPacket passes with flag
- `load_test_denied_without_policy_flag` - LoadTest blocked by default
- `load_test_allowed_with_policy_flag` - LoadTest passes with flag
- `remote_execution_denied_by_default` - RemoteExecution blocked by default
- `remote_execution_allowed_with_policy_flag` - RemoteExecution passes with flag
- `json_mode_denial_includes_structured_data` - JSON denial has decision_id, operation_risk, denied_reasons
- `human_mode_denial_is_readable` - Human denial contains "DENIED"
- `denied_public_target_out_of_scope` - Out-of-scope target denied
- `allowed_target_passes_scope_check` - In-scope target allowed
- `exploit_adjacent_denied_by_default` - ExploitAdjacent blocked by default
- `credential_testing_denied_by_default` - CredentialTesting blocked by default
- `credential_testing_allowed_with_policy_flag` - CredentialTesting passes with flag

## Notes

- All handler migrations compile and pass tests under default features.
- Feature-gate build failures are pre-existing on main and unrelated to this change.
- The NSE sandbox test failure is pre-existing and unrelated.
- Pre-existing clippy warnings (18 total) are unchanged by this work.
