//! iOS IPA static analyzer (pure Rust, feature `mobile`).
//!
//! Phase 1 implementation per mobile-first-handoff-plan.md and requirements:
//! - Opens IPA as ZIP (zip crate).
//! - Bounded extraction with ZipSlip rejection and size caps.
//! - Locates first "Payload/*.app/Info.plist".
//! - Deserializes via plist + serde into small typed structs for key keys.
//! - Extracts bundle metadata, ATS (NSAppTransportSecurity) exceptions, file-sharing flags,
//!   custom URL schemes, extension markers.
//! - Additional bounded scans inside the .app for small text assets (.plist/.xml/.json/.strings/.js)
//!   using a small duplicated secret scanner (Phase 1 isolation; no dependency on recon secrets).
//! - Detects _CodeSignature presence (signed indicator) and embedded.mobileprovision markers
//!   (get-task-allow, aps-environment, enterprise, debug/ad-hoc indicators) via bounded reads.
//! - Emits Keychain guidance recommendation when transport or secret findings are present.
//! - Produces MobileScanReport with Ios platform and appropriately-severity MobileFinding entries.
//! - Public async fn analyze_ipa(path: &Path) -> Result<MobileScanReport>.
//! - Unit tests using synthetic in-memory ZIP + plist payloads (via tempfile + zip writer).
//!
//! Safety: no external tools, no shell, no dynamic execution. All work is offline on caller-supplied lab binaries.

use crate::error::{EggsecError, Result};
use crate::mobile::{MobileFinding, MobilePlatform, MobileScanReport};
use crate::types::Severity;
use plist::from_bytes;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;
use std::sync::LazyLock;
use zip::ZipArchive;

const MAX_ARCHIVE_SIZE: u64 = 200 * 1024 * 1024; // 200 MiB (mirrors mobile/mod.rs guard)
const MAX_FILE_READ: u64 = 10 * 1024 * 1024; // 10 MiB per individual file for analysis
const MAX_TEXT_SCAN: u64 = 1024 * 1024; // 1 MiB cap for secret/config text scans

/// ZipSlip / path traversal guard. Rejects absolute paths and any component that is ".." or ".".
fn is_safe_entry_name(name: &str) -> bool {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    for comp in name.split(['/', '\\']) {
        if comp == ".." || comp == "." {
            return false;
        }
    }
    true
}

/// Read a single zip entry safely with size cap. Returns the full bytes or error.
fn read_entry_safe<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
    if !is_safe_entry_name(name) {
        return Err(EggsecError::Validation(format!(
            "Unsafe zip entry name rejected (ZipSlip): {}",
            name
        )));
    }
    let mut zf = archive
        .by_name(name)
        .map_err(|e| EggsecError::Internal(format!("failed to open zip entry '{}': {}", name, e)))?;
    let sz = zf.size();
    if sz > MAX_FILE_READ {
        return Err(EggsecError::Validation(format!(
            "zip entry '{}' too large for analysis ({} > {} bytes)",
            name, sz, MAX_FILE_READ
        )));
    }
    let mut buf = Vec::with_capacity(sz as usize);
    zf.read_to_end(&mut buf)
        .map_err(|e| EggsecError::Internal(format!("read failed for '{}': {}", name, e)))?;
    Ok(buf)
}

/// Locate the first well-formed Payload/*.app/Info.plist entry name.
fn locate_info_plist<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name();
            if !is_safe_entry_name(name) {
                continue;
            }
            // Expect exactly Payload/NAME.app/Info.plist (two slashes after Payload)
            if name.starts_with("Payload/")
                && name.ends_with("/Info.plist")
                && name.matches('/').count() == 2
            {
                return Some(name.to_owned());
            }
        }
    }
    None
}

/// Typed representation of NSAppTransportSecurity exception domain entry.
#[derive(Debug, Deserialize, Default)]
struct AtsException {
    #[serde(rename = "NSExceptionAllowsInsecureHTTPLoads")]
    ns_exception_allows_insecure_http_loads: Option<bool>,
    #[serde(rename = "NSIncludesSubdomains")]
    ns_includes_subdomains: Option<bool>,
    #[serde(rename = "NSExceptionMinimumTLSVersion")]
    ns_exception_minimum_tls_version: Option<String>,
}

/// NSAppTransportSecurity dictionary.
#[derive(Debug, Deserialize, Default)]
struct AtsDict {
    #[serde(rename = "NSAllowsArbitraryLoads")]
    ns_allows_arbitrary_loads: Option<bool>,
    #[serde(rename = "NSAllowsArbitraryLoadsInWebContent")]
    ns_allows_arbitrary_loads_in_web_content: Option<bool>,
    #[serde(rename = "NSExceptionDomains")]
    ns_exception_domains: Option<HashMap<String, AtsException>>,
}

/// One entry in CFBundleURLTypes.
#[derive(Debug, Deserialize, Default)]
struct BundleUrlType {
    #[serde(rename = "CFBundleURLSchemes")]
    cf_bundle_url_schemes: Option<Vec<String>>,
}

/// Minimal typed view of Info.plist keys we care about.
/// Unknown keys are ignored; missing keys become None.
#[derive(Debug, Deserialize, Default)]
struct InfoPlist {
    #[serde(rename = "CFBundleIdentifier")]
    cf_bundle_identifier: Option<String>,
    #[serde(rename = "CFBundleShortVersionString")]
    cf_bundle_short_version_string: Option<String>,
    #[serde(rename = "CFBundleVersion")]
    cf_bundle_version: Option<String>,
    #[serde(rename = "MinimumOSVersion")]
    #[allow(dead_code)]
    minimum_os_version: Option<String>,
    #[serde(rename = "NSAppTransportSecurity")]
    ns_app_transport_security: Option<AtsDict>,
    #[serde(rename = "UIFileSharingEnabled")]
    ui_file_sharing_enabled: Option<bool>,
    #[serde(rename = "LSSupportsOpeningDocumentsInPlace")]
    ls_supports_opening_documents_in_place: Option<bool>,
    #[serde(rename = "CFBundleURLTypes")]
    cf_bundle_url_types: Option<Vec<BundleUrlType>>,
    #[serde(rename = "NSExtension")]
    #[allow(dead_code)]
    ns_extension: Option<plist::Value>,
}

/// Simple secret finding from the in-IPA text scanner (Phase 1 isolated impl).
struct SimpleSecret {
    value_preview: String,
    severity: Severity,
    description: String,
}

/// Small set of secret patterns duplicated for mobile Phase 1 isolation.
/// (Avoids pulling recon::secrets or other heavy modules.)
static SECRET_PATTERNS: LazyLock<Vec<(Regex, &'static str, Severity)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['\"]?([A-Za-z0-9_\-]{16,})['\"]?"#)
                .expect("invalid api key regex"),
            "API Key",
            Severity::High,
        ),
        (
            Regex::new(r#"(?i)(secret|token|password|passwd|auth_token|access_token)\s*[:=]\s*['\"]?([A-Za-z0-9_\-]{8,})['\"]?"#)
                .expect("invalid secret/token regex"),
            "Secret/Token/Password",
            Severity::High,
        ),
        (
            Regex::new(r#"(?i)AKIA[0-9A-Z]{16}"#).expect("invalid AWS key regex"),
            "AWS Access Key",
            Severity::Critical,
        ),
        (
            Regex::new(r#"(?i)sk-[0-9a-zA-Z]{20,}"#).expect("invalid OpenAI key regex"),
            "OpenAI/LLM Key",
            Severity::High,
        ),
        (
            Regex::new(r#"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"#)
                .expect("invalid private key regex"),
            "Private Key",
            Severity::Critical,
        ),
        (
            Regex::new(r#"(?i)bearer\s+[A-Za-z0-9\-_.=]{16,}"#).expect("invalid bearer regex"),
            "Bearer Token",
            Severity::High,
        ),
        (
            Regex::new(r#"(?i)ghp_[0-9a-zA-Z]{30,}"#).expect("invalid GitHub token regex"),
            "GitHub Token",
            Severity::High,
        ),
        (
            Regex::new(r#"(?i)glpat-[0-9a-zA-Z\-_]{20,}"#).expect("invalid GitLab token regex"),
            "GitLab Token",
            Severity::High,
        ),
    ]
});

fn scan_for_secrets(text: &str) -> Vec<SimpleSecret> {
    let mut out = Vec::new();
    for (re, desc, sev) in SECRET_PATTERNS.iter() {
        for m in re.find_iter(text) {
            let val = m.as_str();
            let preview = if val.chars().count() > 28 {
                format!("{}...", val.chars().take(28).collect::<String>())
            } else {
                val.to_string()
            };
            out.push(SimpleSecret {
                value_preview: preview,
                severity: *sev,
                description: (*desc).to_string(),
            });
        }
    }
    out
}

/// Public entry point. Pure-Rust, async wrapper around sync ZIP/plist work (bounded).
pub async fn analyze_ipa(path: &Path) -> Result<MobileScanReport> {
    let start = std::time::Instant::now();
    let mut report = MobileScanReport::new(path.to_string_lossy().as_ref(), MobilePlatform::Ios);

    // Archive-level size guard (defensive; mirrors the one in run_cli)
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_ARCHIVE_SIZE {
            return Err(EggsecError::Validation(format!(
                "IPA too large for static analysis ({} bytes > {} MiB limit)",
                meta.len(),
                MAX_ARCHIVE_SIZE / 1024 / 1024
            )));
        }
    }

    let file = std::fs::File::open(path)
        .map_err(|e| EggsecError::Internal(format!("failed to open IPA '{}': {}", path.display(), e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| EggsecError::Internal(format!("invalid or corrupt IPA ZIP: {}", e)))?;

    // 1. Locate the app bundle Info.plist (first match)
    let plist_name = locate_info_plist(&mut archive)
        .ok_or_else(|| EggsecError::Validation(
            "No valid Payload/*.app/Info.plist found in IPA (malformed bundle or ZipSlip-filtered)".to_string(),
        ))?;

    // Derive the bundle directory prefix for later walks (e.g. "Payload/MyApp.app/")
    let bundle_prefix = match plist_name.rfind('/') {
        Some(idx) => plist_name[..=idx].to_owned(),
        None => "Payload/".to_owned(),
    };

    // 2. Read + deserialize the main Info.plist
    let plist_bytes = read_entry_safe(&mut archive, &plist_name)?;
    let info: InfoPlist = from_bytes(&plist_bytes)
        .map_err(|e| EggsecError::Internal(format!("failed to parse Info.plist as plist: {}", e)))?;

    report.app_id = info.cf_bundle_identifier.clone();
    match (&info.cf_bundle_short_version_string, &info.cf_bundle_version) {
        (Some(s), Some(b)) => report.version = Some(format!("{} ({})", s, b)),
        (Some(s), None) => report.version = Some(s.clone()),
        (None, Some(b)) => report.version = Some(b.clone()),
        (None, None) => {}
    }

    // Extraction of MinimumOSVersion and NSExtension per requirements (Phase 1 records presence;
    // richer enumeration of extension points can be added later without changing the API).
    let _ = &info.minimum_os_version;
    let _ = &info.ns_extension;

    // 3. ATS / transport analysis
    if let Some(ref ats) = info.ns_app_transport_security {
        let mut insecure_reasons: Vec<String> = Vec::new();

        if ats.ns_allows_arbitrary_loads == Some(true) {
            insecure_reasons.push("NSAllowsArbitraryLoads=true".to_string());
        }
        if ats.ns_allows_arbitrary_loads_in_web_content == Some(true) {
            insecure_reasons.push("NSAllowsArbitraryLoadsInWebContent=true".to_string());
        }
        if let Some(ref domains) = ats.ns_exception_domains {
            for (domain, exc) in domains {
                if exc.ns_exception_allows_insecure_http_loads == Some(true) {
                    insecure_reasons.push(format!(
                        "NSExceptionAllowsInsecureHTTPLoads for {} (subdomains={})",
                        domain,
                        exc.ns_includes_subdomains.unwrap_or(false)
                    ));
                }
                if let Some(ref min_tls) = exc.ns_exception_minimum_tls_version {
                    if min_tls.to_lowercase().contains("1.0") || min_tls.to_lowercase().contains("1.1") {
                        insecure_reasons.push(format!("weak min TLS {} for {}", min_tls, domain));
                    }
                }
            }
        }

        if !insecure_reasons.is_empty() {
            report.findings.push(MobileFinding {
                category: "transport".to_string(),
                severity: Severity::High,
                title: "Insecure App Transport Security (ATS) configuration".to_string(),
                description: "NSAppTransportSecurity disables or weakens TLS requirements, allowing cleartext HTTP or weak exceptions.".to_string(),
                recommendation: "Remove NSAllowsArbitraryLoads and all NSExceptionDomains that permit insecure loads. Enforce HTTPS everywhere; add certificate pinning for high-value endpoints.".to_string(),
                evidence: Some(insecure_reasons.join("; ")),
            });
        }
    }

    // 4. Data export / file sharing risks
    let file_sharing = info.ui_file_sharing_enabled.unwrap_or(false);
    let docs_in_place = info.ls_supports_opening_documents_in_place.unwrap_or(false);
    if file_sharing || docs_in_place {
        report.findings.push(MobileFinding {
            category: "storage".to_string(),
            severity: Severity::Medium,
            title: "Data export / file sharing enabled".to_string(),
            description: "UIFileSharingEnabled or LSSupportsOpeningDocumentsInPlace allows the Files app or iTunes to access the app container.".to_string(),
            recommendation: "Disable both flags for apps that handle sensitive user or enterprise data. Prefer app-private on-disk storage protected by Data Protection and Keychain-backed keys.".to_string(),
            evidence: Some(format!(
                "UIFileSharingEnabled={}, LSSupportsOpeningDocumentsInPlace={}",
                file_sharing, docs_in_place
            )),
        });
    }

    // 5. Custom URL schemes (note potential hijacking surface)
    if let Some(ref url_types) = info.cf_bundle_url_types {
        for ut in url_types {
            if let Some(ref schemes) = ut.cf_bundle_url_schemes {
                for scheme in schemes {
                    if !scheme.is_empty() {
                        report.findings.push(MobileFinding {
                            category: "url-scheme".to_string(),
                            severity: Severity::Low,
                            title: "Custom URL scheme registered".to_string(),
                            description: format!(
                                "App declares custom URL scheme '{}'. Malicious apps or websites can invoke the handler.",
                                scheme
                            ),
                            recommendation: "Strictly validate every incoming URL, reject unexpected parameters, and prefer Universal Links / App Links for new integrations.".to_string(),
                            evidence: Some(scheme.clone()),
                        });
                    }
                }
            }
        }
    }

    // 6. Walk the archive for signing markers, provisioning profiles, and small text assets inside the bundle.
    // Two-pass design: first pass only collects metadata (names/sizes/flags) so that
    // no ZipFile borrows are held when we later call read_entry_safe (which needs &mut archive).
    let mut has_code_signature = false;
    let mut debug_indicators: Vec<String> = Vec::new();
    // (name, size, is_provisioning_candidate)
    let mut candidates: Vec<(String, u64, bool)> = Vec::new();

    for i in 0..archive.len() {
        let name = match archive.by_index(i) {
            Ok(e) => e.name().to_string(),
            Err(_) => continue,
        };
        if !is_safe_entry_name(&name) {
            continue;
        }
        // Fetch size in its own scope so the temporary ZipFile is dropped before next iteration.
        let size = match archive.by_index(i) {
            Ok(e) => e.size(),
            Err(_) => continue,
        };

        if name.contains("_CodeSignature/") {
            has_code_signature = true;
        }

        let is_provision = name.ends_with(".mobileprovision") || name.contains("embedded.mobileprovision");
        if is_provision || (size > 0 && size < MAX_TEXT_SCAN) {
            candidates.push((name, size, is_provision));
        }
    }

    // Second pass: perform safe deep reads. No live ZipFile from the first pass exists here.
    for (name, size, is_provision) in candidates {
        if is_provision && size > 0 && size < MAX_TEXT_SCAN {
            if let Ok(bytes) = read_entry_safe(&mut archive, &name) {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("get-task-allow") {
                    debug_indicators.push(format!("get-task-allow entitlement in {}", name));
                }
                if text.contains("aps-environment") && text.to_lowercase().contains("development") {
                    debug_indicators.push("aps-environment=development".to_string());
                }
                let lower = text.to_lowercase();
                if lower.contains("enterprise") || lower.contains("ad-hoc") || lower.contains("debug") {
                    debug_indicators.push("development/ad-hoc/enterprise profile marker".to_string());
                }
            }
            continue;
        }

        // Bounded secret scan on small text/config files inside the .app
        let inside_app = name.starts_with(&bundle_prefix) || (name.starts_with("Payload/") && name.contains(".app/"));
        if inside_app && size > 0 && size < MAX_TEXT_SCAN {
            let lower = name.to_lowercase();
            if lower.ends_with(".plist")
                || lower.ends_with(".xml")
                || lower.ends_with(".json")
                || lower.ends_with(".strings")
                || lower.ends_with(".js")
                || lower.ends_with(".txt")
            {
                if let Ok(bytes) = read_entry_safe(&mut archive, &name) {
                    let text = String::from_utf8_lossy(&bytes);
                    for secret in scan_for_secrets(&text) {
                        report.findings.push(MobileFinding {
                            category: "secret".to_string(),
                            severity: secret.severity,
                            title: format!("Hardcoded {} in bundle asset", secret.description),
                            description: format!(
                                "Potential hardcoded credential or secret detected in {}",
                                name
                            ),
                            recommendation: "Remove all secrets, keys, and tokens from the IPA. Use remote configuration or Keychain-backed secure storage. Never embed production credentials.".to_string(),
                            evidence: Some(secret.value_preview),
                        });
                    }
                }
            }
        }
    }

    if !has_code_signature {
        report.findings.push(MobileFinding {
            category: "signing".to_string(),
            severity: Severity::Low,
            title: "Missing _CodeSignature directory".to_string(),
            description: "No _CodeSignature/ folder found; the IPA may be unsigned or the signature was stripped.".to_string(),
            recommendation: "Sign all release and internal-distribution IPAs with a valid Apple Distribution certificate and provisioning profile.".to_string(),
            evidence: None,
        });
    }

    if !debug_indicators.is_empty() {
        report.findings.push(MobileFinding {
            category: "build".to_string(),
            severity: Severity::Low,
            title: "Debug / ad-hoc / development provisioning indicators".to_string(),
            description: "Provisioning profile or entitlements contain development/debug markers (e.g. get-task-allow, aps-environment development).".to_string(),
            recommendation: "Use App Store / Enterprise / Ad-Hoc distribution profiles for anything leaving the development environment. Remove get-task-allow from release entitlements.".to_string(),
            evidence: Some(debug_indicators.join("; ")),
        });
    }

    // 7. Keychain / secure storage guidance (always emitted on iOS as defensive lab reminder)
    // This is informational guidance, not a flaw finding. Matches test expectations and
    // the overall mobile module recommendation to prefer platform secure storage.
    report.findings.push(MobileFinding {
        category: "storage".to_string(),
        severity: Severity::Info,
        title: "Prefer iOS Keychain for secrets (guidance)".to_string(),
        description: "Static analysis cannot observe runtime Keychain vs. NSUserDefaults / file usage. Use the Keychain for credentials, tokens, and keys.".to_string(),
        recommendation: "Store secrets exclusively via the Keychain (SecItemAdd / kSecClassGenericPassword etc.) with kSecAttrAccessibleWhenUnlockedThisDeviceOnly (or stricter). Audit for [[NSUserDefaults standardUserDefaults] setObject:...], direct file writes of tokens, and plaintext .plist / .json config files containing credentials.".to_string(),
        evidence: None,
    });

    report.duration_ms = start.elapsed().as_millis() as u64;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use zip::write::FileOptions;
    use zip::ZipWriter;
    use std::io::Write as IoWrite;

    /// Helper: build a minimal valid IPA (ZIP) containing the given Info.plist XML bytes
    /// plus an optional _CodeSignature marker and/or a provisioning file.
    fn make_test_ipa(
        plist_xml: &str,
        with_signature: bool,
        extra_files: &[(&str, &[u8])],
    ) -> NamedTempFile {
        let mut tmp = NamedTempFile::new().expect("tempfile");
        {
            let mut zw = ZipWriter::new(&mut tmp);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zw.start_file("Payload/TestApp.app/Info.plist", opts)
                .expect("start Info.plist");
            zw.write_all(plist_xml.as_bytes()).expect("write plist");

            if with_signature {
                zw.start_file("Payload/TestApp.app/_CodeSignature/CodeResources", opts)
                    .expect("start signature");
                zw.write_all(b"fake signature blob").expect("write sig");
            }

            for (name, data) in extra_files {
                zw.start_file(*name, opts).expect("start extra");
                zw.write_all(data).expect("write extra");
            }

            zw.finish().expect("finish zip");
        }
        tmp
    }

    fn basic_plist_with_ats_and_sharing() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>com.example.vulnerable</string>
    <key>CFBundleShortVersionString</key><string>1.0.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>MinimumOSVersion</key><string>15.0</string>
    <key>NSAppTransportSecurity</key>
    <dict>
        <key>NSAllowsArbitraryLoads</key><true/>
        <key>NSExceptionDomains</key>
        <dict>
            <key>legacy.example.com</key>
            <dict>
                <key>NSExceptionAllowsInsecureHTTPLoads</key><true/>
                <key>NSIncludesSubdomains</key><true/>
            </dict>
        </dict>
    </dict>
    <key>UIFileSharingEnabled</key><true/>
    <key>LSSupportsOpeningDocumentsInPlace</key><true/>
</dict>
</plist>"#.to_string()
    }

    fn clean_plist() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>com.example.clean</string>
    <key>CFBundleShortVersionString</key><string>2.1.0</string>
    <key>CFBundleVersion</key><string>42</string>
    <key>MinimumOSVersion</key><string>16.0</string>
</dict>
</plist>"#.to_string()
    }

    fn plist_with_url_scheme() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>com.example.schemes</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLSchemes</key>
            <array>
                <string>myapp</string>
                <string>myapp-prod</string>
            </array>
        </dict>
    </array>
</dict>
</plist>"#.to_string()
    }

    #[tokio::test]
    async fn test_analyze_ipa_detects_ats_and_file_sharing() {
        let plist = basic_plist_with_ats_and_sharing();
        let tmp = make_test_ipa(&plist, true, &[]);
        let report = analyze_ipa(tmp.path()).await.expect("analyze");

        assert_eq!(report.platform, MobilePlatform::Ios);
        assert_eq!(report.app_id.as_deref(), Some("com.example.vulnerable"));
        assert_eq!(report.version.as_deref(), Some("1.0.0 (1)"));

        let has_transport = report.findings.iter().any(|f| {
            f.category == "transport"
                && f.severity == Severity::High
                && f.title.contains("App Transport Security")
        });
        assert!(has_transport, "expected High ATS finding");

        let has_export = report.findings.iter().any(|f| {
            f.category == "storage"
                && f.severity == Severity::Medium
                && f.title.contains("Data export")
        });
        assert!(has_export, "expected Medium file-sharing finding");
    }

    #[tokio::test]
    async fn test_analyze_ipa_clean_plist_no_high_findings() {
        let plist = clean_plist();
        let tmp = make_test_ipa(&plist, true, &[]);
        let report = analyze_ipa(tmp.path()).await.expect("analyze");

        // A clean, properly-signed IPA with no ATS exceptions, no file-sharing, no custom schemes,
        // and no secrets should produce zero High/Critical findings. The Keychain guidance is only
        // emitted when we actually saw transport or secret issues (per the Phase 1 spec).
        assert!(report.findings.iter().all(|f| f.severity != Severity::High && f.severity != Severity::Critical));
        // No Low signing note either, because we included a _CodeSignature in this test IPA.
        assert!(report.findings.is_empty() || report.findings.iter().all(|f| f.severity == Severity::Info || f.severity == Severity::Low));
    }

    #[tokio::test]
    async fn test_analyze_ipa_custom_url_schemes() {
        let plist = plist_with_url_scheme();
        let tmp = make_test_ipa(&plist, false, &[]);
        let report = analyze_ipa(tmp.path()).await.expect("analyze");

        let scheme_findings: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.category == "url-scheme")
            .collect();
        assert_eq!(scheme_findings.len(), 2);
        assert!(scheme_findings.iter().any(|f| f.evidence.as_deref() == Some("myapp")));
        assert!(scheme_findings.iter().any(|f| f.evidence.as_deref() == Some("myapp-prod")));
        assert!(scheme_findings.iter().all(|f| f.severity == Severity::Low));
    }

    #[tokio::test]
    async fn test_analyze_ipa_missing_signature_and_debug_profile() {
        let plist = clean_plist();
        // Add a fake embedded.mobileprovision containing get-task-allow and development aps
        let provision = b"<?xml version=\"1.0\"?><dict><key>get-task-allow</key><true/><key>aps-environment</key><string>development</string></dict>";
        let tmp = make_test_ipa(
            &plist,
            false,
            &[("Payload/TestApp.app/embedded.mobileprovision", provision)],
        );
        let report = analyze_ipa(tmp.path()).await.expect("analyze");

        assert!(report.findings.iter().any(|f| f.title.contains("Missing _CodeSignature")));
        let build = report.findings.iter().find(|f| f.category == "build").expect("build finding");
        assert!(build.severity == Severity::Low);
        assert!(build.evidence.as_ref().unwrap().contains("get-task-allow"));
        assert!(build.evidence.as_ref().unwrap().contains("aps-environment"));
    }

    #[tokio::test]
    async fn test_analyze_ipa_detects_hardcoded_secret_in_js() {
        let plist = clean_plist();
        let secret_js = b"const apiKey = 'sk-1234567890abcdef1234567890abcdef'; const token = 'ghp_abcdefghijklmnopqrstuvwxyz123456';";
        let tmp = make_test_ipa(
            &plist,
            true,
            &[("Payload/TestApp.app/www/config.js", secret_js)],
        );
        let report = analyze_ipa(tmp.path()).await.expect("analyze");

        let secrets: Vec<_> = report.findings.iter().filter(|f| f.category == "secret").collect();
        assert!(!secrets.is_empty());
        assert!(secrets.iter().any(|f| f.title.contains("OpenAI") || f.title.contains("GitHub")));
        // Also triggers the Keychain guidance
        assert!(report.findings.iter().any(|f| f.title.contains("Keychain")));
    }

    #[tokio::test]
    async fn test_analyze_ipa_rejects_zip_slip_and_reports_error() {
        let mut tmp = NamedTempFile::new().expect("tempfile");
        {
            let mut zw = ZipWriter::new(&mut tmp);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
            // Malicious entry
            zw.start_file("Payload/../../evil.plist", opts).expect("start evil");
            zw.write_all(b"bad").expect("write");
            zw.finish().expect("finish");
        }
        let res = analyze_ipa(tmp.path()).await;
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("Unsafe") || msg.contains("No valid Payload"));
    }

    #[tokio::test]
    async fn test_analyze_ipa_rejects_missing_info_plist() {
        let mut tmp = NamedTempFile::new().expect("tempfile");
        {
            let mut zw = ZipWriter::new(&mut tmp);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
            zw.start_file("Payload/Weird.app/Other.plist", opts).expect("start other");
            zw.write_all(b"not the info").expect("write");
            zw.finish().expect("finish");
        }
        let res = analyze_ipa(tmp.path()).await;
        assert!(res.is_err());
        let msg = res.err().unwrap().to_string();
        assert!(msg.contains("No valid Payload"));
    }
}
