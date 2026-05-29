# Slapper Public Repository Polish Plan

This plan prepares Slapper for public release without changing its core purpose. The goal is to make the repository legally clean, technically reproducible, narratively coherent, and harder to misread as an unscoped offensive toolkit.

Slapper should be positioned as a Rust-native, scope-enforced security assessment and defense-validation engine for authorized testing, local lab validation, WAF regression, CI security checks, and agent-readable security workflows.

Do not add new offensive capability as part of this plan. Prefer documentation cleanup, safety defaults, metadata correctness, build reproducibility, CI hardening, and honest feature labeling.

## Implementation principles

1. Preserve existing functionality unless a command, document, or metadata entry is clearly stale, misleading, unsafe by default, or inconsistent with current project direction.
2. Make scope enforcement and dry-run planning the default public story.
3. Move high-risk examples out of the README and into clearly labeled advanced/lab-only documentation.
4. Do not claim infrastructure, domains, emails, security SLAs, audits, releases, or organizations that do not actually exist.
5. Keep NSE support as optional compatibility. Do not reintroduce arbitrary Python/Ruby plugin runtimes.
6. Prefer small, reviewable commits grouped by phase.

## Phase 1: Repository identity and metadata cleanup

### 1.1 Fix repository URLs

Update all stale references to `https://github.com/slapper-tool/slapper` unless that organization and repository are intentionally being created before publication.

Likely files:

- `Cargo.toml`
- `crates/slapper/Cargo.toml`
- `README.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
- Any docs under `docs/`
- Any generated examples or config templates

Use the actual public repository URL. If the repo will remain under the current owner, use:

```text
https://github.com/dbowm91/slapper
```

Acceptance criteria:

- `rg "slapper-tool|slapper.dev|slapper-tool.org"` returns no stale references unless they are intentionally documented as future placeholders.
- Cargo workspace `repository`, crate `homepage`, crate `documentation`, README clone instructions, CONTRIBUTING upstream remote, and SECURITY advisory links all agree.

### 1.2 Normalize crate metadata

Update `crates/slapper/Cargo.toml` package metadata so it reflects the public positioning.

Current framing is too generic/offensive: “High-performance security testing toolkit for penetration testers and security researchers.” Replace with something closer to:

```toml
description = "Scope-enforced Rust security assessment engine for defense validation and regression testing"
```

Review keywords. Prefer keywords such as:

```toml
keywords = ["security", "defense-validation", "waf", "scanner", "testing"]
```

Avoid over-indexing on terms such as `pentesting`, `fuzzer`, or `vulnerability-scanner` if the rest of the public docs are being reframed around authorized defense validation.

Acceptance criteria:

- Crate metadata is consistent with README positioning.
- Metadata does not imply unscoped exploitation or arbitrary offensive automation.

### 1.3 Align Rust version documentation

The workspace declares `rust-version = "1.80"`. `CONTRIBUTING.md` currently says Rust 1.70 or later. Update docs to match the workspace.

Acceptance criteria:

- `rg "1\.70|Rust 1\.70"` returns no stale minimum-version claim.
- README, CONTRIBUTING, and Cargo metadata agree on MSRV.

## Phase 2: Legal and governance files

### 2.1 Add license files

The workspace declares `MIT OR Apache-2.0`. Add conventional root license files:

- `LICENSE-MIT`
- `LICENSE-APACHE`

Optionally add a short `LICENSE` file that explains the dual-license choice:

```text
Licensed under either of Apache License, Version 2.0 or MIT license at your option.
See LICENSE-APACHE and LICENSE-MIT for details.
```

Use the canonical MIT and Apache-2.0 texts. Do not invent modified license language.

Acceptance criteria:

- Root license files exist.
- Cargo license fields still say `MIT OR Apache-2.0`.
- `cargo package -p slapper --list` includes the relevant license files or package metadata remains valid.

### 2.2 Add or remove Code of Conduct references

`CONTRIBUTING.md` references a Code of Conduct and a conduct email. Either add a real `CODE_OF_CONDUCT.md` or remove/soften the claim.

Recommended: add `CODE_OF_CONDUCT.md` using Contributor Covenant or a concise project-specific conduct policy. Do not list an email address unless it is controlled.

Acceptance criteria:

- `CONTRIBUTING.md` links to an existing `CODE_OF_CONDUCT.md`, or the reference is removed.
- No nonfunctional conduct email remains.

### 2.3 Rewrite SECURITY.md for pre-1.0 honesty

Current SECURITY.md reads like a mature organization policy with fixed response timelines, backport policy, advisory URLs, and domain/email references. Rewrite it to be accurate for a pre-public or early public project.

Required content:

- Authorized-use policy.
- How to report vulnerabilities.
- Preferred private reporting channel. Use GitHub private vulnerability reporting only if enabled for the actual repo.
- Scope controls and safe operation guidance.
- Sensitive-data handling guidance.
- No formal SLA unless the maintainer is willing to honor it.
- No claims about backporting the last two major versions unless release branches exist.
- No claims about regular audits unless there is an actual audit process.

Remove or revise:

- `security@slapper-tool.org` unless the domain exists and is controlled.
- `https://github.com/slapper-tool/slapper/security/advisories/new` unless that is the real repo.
- PGP key URL unless real.
- “All known vulnerabilities have been fixed” unless backed by current audit output.
- Fixed vulnerability list if it references packages no longer present or future/unverified RustSec IDs.

Acceptance criteria:

- SECURITY.md is modest, accurate, and repo-specific.
- No broken domain/email/advisory links remain.
- The policy does not overpromise response timelines or backport support.

## Phase 3: README restructure

### 3.1 Replace README landing structure

Rewrite README into a shorter public landing page. Move the long command reference into `docs/cli.md` or `docs/usage.md`.

Recommended README structure:

1. Title and one-paragraph positioning.
2. “What Slapper is” section.
3. “What Slapper is not” section.
4. Safety model: scope file, config, rate limits, dry-run planning.
5. Quick start using localhost or a deliberately safe target only.
6. Core workflows:
   - Scoped web/security assessment.
   - WAF/defense validation in a lab.
   - CI regression checks.
   - Agent/MCP integration.
   - Optional NSE compatibility.
7. Feature flags table with status labels.
8. Installation/build instructions.
9. Documentation links.
10. Responsible-use notice.

Do not keep the entire command encyclopedia in the README.

Acceptance criteria:

- README is concise enough to serve as a landing page.
- README foregrounds authorized scope, dry-run planning, and safe defaults.
- README no longer presents high-risk examples before explaining the safety model.

### 3.2 Add “What Slapper is not”

Add a section with language similar to:

```markdown
## What Slapper is not

Slapper is not an exploitation framework, botnet component, credential attack platform, or tool for unscoped internet scanning. Some modules can generate aggressive traffic or security-test payloads, so advanced capabilities are feature-gated and intended for systems you own, operate, or have explicit authorization to test.
```

Acceptance criteria:

- README clearly distinguishes Slapper from Metasploit-like exploitation frameworks and unscoped scanners.
- This language appears before advanced feature examples.

### 3.3 Replace public examples with safe workflow examples

Use examples like:

```bash
cargo build -p slapper

slapper --generate-config > slapper.toml

slapper config validate --config slapper.toml

slapper plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080

slapper scan 127.0.0.1 --profile quick --scope examples/scope-localhost.toml --json
```

Only use public domains like `example.com` for non-invasive examples, and prefer localhost for anything involving fuzzing, WAF testing, stress testing, authentication testing, or endpoint discovery.

Acceptance criteria:

- README quick start uses localhost or an explicit lab target.
- README examples show `--scope` for nontrivial operations.
- README examples show `plan`/dry-run before execution.

### 3.4 Move advanced/high-risk examples into lab-only docs

Create or update:

- `docs/lab-safety.md`
- `docs/advanced-features.md`
- `docs/cli.md`

Move the following categories out of the README:

- Stress/flood testing.
- Proxy pool/Tor examples.
- WAF bypass/evasion examples.
- Auth brute-force or credential-stuffing examples.
- Distributed scanning examples.
- Raw packet operations.

Each advanced section must include:

- Authorization warning.
- Scope requirement.
- Rate/concurrency guidance.
- Local/private lab recommendation.
- Feature flag needed.

Acceptance criteria:

- High-risk examples are not in the top-level README.
- Advanced docs label these workflows as authorized lab/defense-validation only.

## Phase 4: Safety defaults and CLI language audit

### 4.1 Audit public CLI help strings

Review CLI help strings under:

- `crates/slapper/src/cli/mod.rs`
- `crates/slapper/src/cli/**/*.rs`

Soften language where it reads as offensive-first. Examples:

- Rename comment group `Attack operations` to `Assessment operations` or `Validation operations`.
- Change help text that says “Detect and bypass WAFs” to “Evaluate WAF detection and evasion resistance.”
- Change “brute force, credential stuffing, MFA” help text to “Validate authentication controls in authorized environments.”

Do not remove technical specificity where it is needed for operator clarity, but avoid making offensive actions sound like the default purpose.

Acceptance criteria:

- `slapper --help` reads as a scoped assessment/defense-validation tool.
- High-risk subcommands are clearly described as validation checks for authorized environments.

### 4.2 Ensure scope-first behavior is documented and, where possible, enforced

Inspect scope-loading and command-dispatch code. If scope is already enforced, document it accurately. If some high-risk commands can run without explicit scope, add TODOs or implement minimal guardrails if straightforward.

Priority commands to check:

- `fuzz`
- `waf`
- `waf-stress`
- `auth-test`
- `stress`
- `proxy`
- `cluster`
- `packet`
- `remote`
- `exec`

Recommended minimal behavior:

- For high-risk commands, warn loudly if no scope file is supplied.
- For the most dangerous commands, require explicit `--scope` unless a config setting intentionally disables this for local development.
- Always allow `plan` and `doctor` without network activity.

Acceptance criteria:

- Documentation and behavior agree.
- No README example suggests running high-risk commands without scope.
- Any behavioral changes include tests or at least command-level validation tests.

### 4.3 Review stealth/proxy language

The CLI includes `--stealth`, proxy auth, bearer tokens, cookies, API keys, user-agent changes, jitter, and rate limits. These are useful for authorized assessment, but public docs should not frame them as evasion for unauthorized testing.

Recommended changes:

- Describe `--stealth` as “randomized timing/header behavior for lab realism and false-positive testing,” or consider renaming in a future breaking release.
- Document proxy usage as enterprise routing, test harnessing, or lab simulation. Keep Tor/proxy-pool material out of the quick start.

Acceptance criteria:

- README does not lead with stealth/proxy features.
- Advanced docs explain legitimate use cases and safety boundaries.

## Phase 5: Feature flags and maturity labels

### 5.1 Create a feature status table

Add a feature status table to README or `docs/features.md` with columns:

- Feature flag
- Status: stable, experimental, stub/planned, lab-only
- Purpose
- Extra dependencies
- Safety notes

Review these flags carefully:

- `stress-testing`
- `packet-inspection`
- `nse`
- `nse-ssh2`
- `nse-sandbox`
- `rest-api`
- `grpc-api`
- `ai-integration`
- `websocket`
- `headless-browser`
- `database`
- `container`
- `sbom`
- `advanced-hunting`
- `compliance`
- `external-integrations`
- `finding-workflow`
- `vuln-management`
- `cloud`
- `git-secrets`
- `wireless`
- `api-schema`
- `pdf`

Do not claim a feature is stable unless it builds and has meaningful implementation/tests.

Acceptance criteria:

- Empty or skeletal features are labeled as experimental/planned/stub.
- The README does not overpromise unsupported capability.

### 5.2 Keep NSE but clarify the plugin boundary

Document that NSE support is optional compatibility with a mature ecosystem, not a general arbitrary plugin runtime.

Add language similar to:

```markdown
Python and Ruby arbitrary plugin runtimes are intentionally not part of Slapper's public extension model. Optional NSE support exists for curated compatibility with Nmap NSE workflows and should be used with sandboxing where possible.
```

Acceptance criteria:

- Docs clearly state that Python/Ruby plugin systems were intentionally removed or are not supported.
- NSE docs recommend `nse-sandbox` where applicable.

## Phase 6: Reproducibility and packaging

### 6.1 Commit Cargo.lock

Remove `Cargo.lock` from `.gitignore` and commit the workspace lockfile.

Rationale: Slapper is primarily a binary/security tool, so a checked-in lockfile improves reproducibility and auditability.

Acceptance criteria:

- `.gitignore` no longer ignores `Cargo.lock`.
- `Cargo.lock` is present at repo root.
- `cargo check -p slapper` works with the committed lockfile.

### 6.2 Check package contents

Run:

```bash
cargo package -p slapper --allow-dirty --list
```

Review whether the package includes unnecessary files or misses required files.

Update `exclude`/`include` in `crates/slapper/Cargo.toml` if needed.

Acceptance criteria:

- Package does not include local binaries, pcaps, test artifacts, secrets, or generated junk.
- Package includes README and license files.

### 6.3 Add installation notes

Document supported install paths:

- Build from source.
- `cargo install --path crates/slapper` for local installs.
- Future crates.io install if intended.
- Optional features install examples.

Do not document crates.io installation unless the crate is actually published or publication is imminent.

Acceptance criteria:

- README install instructions work from a fresh clone.
- Feature-specific build commands are accurate.

## Phase 7: CI and security workflow hardening

### 7.1 Make audit behavior intentional

Review `.github/workflows/test.yml`.

Current security audit uses `cargo audit --deny warnings` with `continue-on-error: true`. Decide whether public main should fail on audit warnings. Recommended for public release:

- Fail on vulnerabilities by default.
- Use an explicit `audit.toml` ignore file only for documented exceptions.
- Do not silently pass failures.

Acceptance criteria:

- Security-audit job behavior is deliberate and documented.
- Any ignored advisory has a reason and review date.

### 7.2 Fix secret scanning

Current workflow attempts `pip install gitLeaks` and skips if unavailable. Replace with a reliable pinned installation or official action.

Recommended options:

- `gitleaks/gitleaks-action`
- Download a pinned release binary.

Acceptance criteria:

- Secret scan actually runs in CI.
- CI fails on detected secrets.
- No best-effort skip unless explicitly limited to forks without permissions.

### 7.3 Validate cargo-deny configuration

If the workflow runs `cargo deny`, ensure the repo includes a valid `deny.toml` or equivalent config if needed.

Acceptance criteria:

- `cargo deny check` works locally.
- License allowlist matches actual dependencies.
- Unmaintained/yanked dependencies are either fixed or documented with exceptions.

### 7.4 Add minimal PR hygiene

Consider adding:

- `pull_request_template.md`
- Issue templates for bug report, feature request, security/safety concern.
- Release checklist.

Acceptance criteria:

- Public contributors have a basic path for reports and changes.
- Security reports are directed away from public issues.

## Phase 8: Documentation additions

### 8.1 Add docs/scope.md

Document the scope model in detail.

Include:

- What a scope file is.
- Allowed domains.
- Excluded domains.
- CIDR ranges.
- Port restrictions.
- Example localhost scope.
- Example internal lab scope.
- How scope is enforced.
- Known limitations.

Acceptance criteria:

- A new user can create a safe scope file without reading source code.
- README links to this doc before advanced commands.

### 8.2 Add examples/scope-localhost.toml

Create a minimal safe scope file for README examples.

Example shape should match the actual config parser, not invented syntax. Inspect current scope parser before writing.

Acceptance criteria:

- Example scope file parses successfully.
- README commands using it are valid.

### 8.3 Add docs/agent-workflows.md

Document agent-oriented workflows without overselling autonomous exploitation.

Recommended sections:

- Why agents use Slapper.
- Tool/API/MCP surfaces.
- Scope-first execution.
- CI/regression usage.
- Scheduled defensive assessments.
- Coding-agent defense-lab usage.
- Output formats for agents: JSON, SARIF, JUnit.
- Human approval boundaries.

Acceptance criteria:

- Agent docs frame Slapper as a controlled assessment backend.
- Docs do not imply an agent should freely scan arbitrary internet targets.

### 8.4 Add docs/lab-safety.md

Document safe use of high-risk features.

Include:

- Stress testing risks.
- Packet/raw socket risks.
- WAF evasion-resistance testing risks.
- Proxy/Tor risks.
- Auth testing risks.
- Rate/concurrency limits.
- Private lab recommendation.
- Monitoring and rollback expectations.

Acceptance criteria:

- All high-risk feature docs link to lab safety.
- README links to lab safety before advanced feature docs.

## Phase 9: Tests and validation

Run the following before final handoff:

```bash
cargo fmt --all -- --check
cargo clippy --lib -p slapper -- -D warnings
cargo check -p slapper
cargo check -p slapper --features rest-api
cargo check -p slapper --features nse
cargo check -p slapper --features nse,nse-sandbox
cargo test --lib -p slapper
```

If time allows, also run:

```bash
cargo check -p slapper --features full
cargo test --test feature_surface -p slapper
cargo deny check
cargo audit
```

If a command fails because of pre-existing unrelated code issues, document the failure exactly in the final handoff notes and do not hide it.

Acceptance criteria:

- Formatting passes.
- At least default `cargo check` passes.
- Any failing checks are documented with exact command output summary.

## Phase 10: Final public-release review

Before making the repo public, perform a final grep and review pass.

Commands:

```bash
rg "slapper-tool|slapper.dev|slapper-tool.org"
rg "brute force|credential stuffing|bypass|stealth|Tor|flood|DDoS|DoS"
rg "TODO|FIXME|reframe-pass|stub|placeholder"
rg "password|token|secret|api[_-]?key|bearer"
```

Interpretation:

- Not every result is bad.
- High-risk terms may be legitimate in code and advanced docs.
- Public landing docs should use these terms carefully and always in scoped/authorized context.
- Placeholder/TODO terms should not appear in prominent README/public claims unless intentionally marked as roadmap.

Acceptance criteria:

- No stale org/domain claims remain.
- No fake security contact remains.
- README is safe and accurate.
- High-risk capability is documented as feature-gated and authorized-use only.
- Feature status is honest.
- Licenses and governance files exist.
- CI is meaningful.
- Repository is ready to switch from private to public.

## Suggested commit sequence

1. `docs: add public repo polish plan`
2. `chore: fix repository metadata and stale URLs`
3. `chore: add license and governance files`
4. `docs: rewrite security policy for pre-1.0 release`
5. `docs: restructure README around scoped defense validation`
6. `docs: add scope, agent workflow, and lab safety docs`
7. `chore: commit Cargo.lock for reproducible builds`
8. `ci: harden audit and secret scanning workflows`
9. `docs: label feature maturity and advanced capabilities`
10. `test: validate public release checks`

## Non-goals

- Do not add new scanners, payloads, bypasses, or stress modules.
- Do not reintroduce Python/Ruby plugin runtimes.
- Do not publish crates or flip repository visibility as part of this plan unless explicitly instructed.
- Do not invent domains, support emails, badge URLs, security contacts, Discord servers, documentation sites, or organizations.
- Do not claim production maturity for experimental or stubbed features.

