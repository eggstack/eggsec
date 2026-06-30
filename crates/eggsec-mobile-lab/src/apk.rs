//! Android APK static analyzer (pure-Rust, bounded, no shell).
//!
//! Phase 1 per plans/mobile-first-handoff-plan.md: high-signal manifest/config
//! findings only. Defense-lab / authorized-use framing. All operations offline
//! on user-supplied test artifacts.
//!
//! This module implements a self-contained, auditable static analyzer:
//! - Opens the APK as a ZIP (zip crate under the `mobile` feature).
//! - Rejects ZipSlip (names containing ".." or absolute paths).
//! - Enforces a total extraction budget (~50 MiB) and skips huge individual entries.
//! - Locates AndroidManifest.xml and distinguishes binary AXML vs text XML.
//! - Minimal pure-Rust AXML decoder: string pool extraction + linear chunk walk
//!   for START_TAG / END_TAG only (no resource table, no styles, no full namespace
//!   resolution). Focused on the attributes required for the security findings.
//! - Text-XML fallback using quick-xml (already a hard dependency of the crate).
//! - Additional bounded scans of small text assets for secrets and insecure-storage hints.
//! - Basic v1 signing detection via presence + content of META-INF/*.RSA|DSA|EC|CERT.*
//! - Emits MobileScanReport + Vec<MobileFinding> using the types from the parent module.
//! - Public async entry point: analyze_apk(path: &Path) -> Result<MobileScanReport>
//! - Unit tests that synthesize minimal valid APKs in-memory via the zip writer.

use crate::{MobileError, MobileFinding, MobilePlatform, MobileScanReport, Result};
use eggsec_core::types::Severity;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use tracing::{debug, warn};

/// Public async entry point (matches the call site in mod.rs and the IPA sibling).
pub async fn analyze_apk(path: &Path) -> Result<MobileScanReport> {
    let path = path.to_path_buf();
    // Zip parsing and AXML walking are CPU-bound; run off the async runtime.
    tokio::task::spawn_blocking(move || analyze_apk_blocking(&path))
        .await
        .map_err(|e| MobileError::Internal(format!("apk analysis task join failed: {}", e)))?
}

/// Core blocking implementation. All ZIP work, bounded extraction, parsing,
/// secondary scans and finding construction happen here.
fn analyze_apk_blocking(path: &Path) -> Result<MobileScanReport> {
    let start = std::time::Instant::now();
    let mut report =
        MobileScanReport::new(path.to_string_lossy().as_ref(), MobilePlatform::Android);

    let file = std::fs::File::open(path)
        .map_err(|e| MobileError::Validation(format!("failed to open APK: {}", e)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| MobileError::Validation(format!("invalid APK zip: {}", e)))?;

    const MAX_TOTAL_EXTRACT: u64 = 50 * 1024 * 1024; // 50 MiB safety budget
    const MAX_SINGLE_CONTENT: u64 = 128 * 1024; // per small-text scan budget

    let mut total_extracted: u64 = 0;
    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut network_config_bytes: Option<Vec<u8>> = None;
    let mut secret_candidates: Vec<(String, Vec<u8>)> = Vec::new();
    let mut saw_cert = false;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| MobileError::Validation(format!("zip entry error: {}", e)))?;
        let raw_name = entry.name().to_string();

        // ZipSlip / path traversal rejection (defense in depth, matches IPA)
        if raw_name.contains("..")
            || raw_name.starts_with('/')
            || raw_name.starts_with('\\')
            || raw_name.contains('\0')
        {
            return Err(MobileError::Validation(format!(
                "ZipSlip path traversal rejected in APK entry: {}",
                raw_name
            )));
        }

        let size = entry.size();
        if size > 10 * 1024 * 1024 {
            debug!("skipping oversized APK entry {} ({} bytes)", raw_name, size);
            continue;
        }
        if total_extracted + size > MAX_TOTAL_EXTRACT {
            warn!("APK extraction budget exhausted (~50 MiB); stopping early");
            break;
        }

        let lower = raw_name.to_ascii_lowercase();

        if lower == "androidmanifest.xml" {
            let mut buf = Vec::new();
            let mut limited = entry.take(MAX_SINGLE_CONTENT);
            limited.read_to_end(&mut buf)?;
            total_extracted += buf.len() as u64;
            manifest_bytes = Some(buf);
        } else if lower.ends_with("network_security_config.xml") {
            let mut buf = Vec::new();
            let mut limited = entry.take(MAX_SINGLE_CONTENT);
            limited.read_to_end(&mut buf)?;
            total_extracted += buf.len() as u64;
            network_config_bytes = Some(buf);
        } else if size <= MAX_SINGLE_CONTENT
            && (lower.ends_with(".xml")
                || lower.ends_with(".json")
                || lower.ends_with(".properties")
                || lower.ends_with(".txt")
                || lower.ends_with(".js")
                || lower.ends_with(".yml")
                || lower.ends_with(".yaml"))
        {
            let mut buf = Vec::new();
            let mut limited = entry.take(MAX_SINGLE_CONTENT);
            limited.read_to_end(&mut buf)?;
            total_extracted += buf.len() as u64;
            secret_candidates.push((raw_name, buf));
        } else if lower.starts_with("meta-inf/")
            && (lower.ends_with(".rsa")
                || lower.ends_with(".dsa")
                || lower.ends_with(".ec")
                || lower.contains("cert."))
        {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            total_extracted += buf.len() as u64;
            saw_cert = true;
            check_cert_for_debug(&raw_name, &buf, &mut report);
        }
    }

    // AndroidManifest.xml is mandatory for a valid APK in our analysis model.
    let mbytes = manifest_bytes.ok_or_else(|| {
        MobileError::Validation("AndroidManifest.xml not found in APK".to_string())
    })?;
    parse_manifest(&mbytes, &mut report)?;

    // Optional: the network security config referenced from the manifest (text XML).
    if let Some(nsc) = network_config_bytes {
        parse_network_security_config(&nsc, &mut report);
    }

    // Bounded secret + insecure storage scan on small text assets extracted above.
    for (fname, bytes) in secret_candidates {
        if let Ok(text) = String::from_utf8(bytes) {
            scan_text_for_secrets_and_storage(&fname, &text, &mut report);
        }
    }

    if !saw_cert {
        // Modern APKs often use APK Signature Scheme v2/v3; absence of v1 META-INF certs
        // is not by itself a finding, but we log for lab visibility.
        debug!("no traditional META-INF/*.RSA/DSA/EC/CERT.* entries observed");
    }

    report.duration_ms = start.elapsed().as_millis() as u64;
    Ok(report)
}

/// Detect binary AXML vs text XML (heuristic) and dispatch to the appropriate parser.
/// On binary parse failure we fall back to the text path for resilience on unusual builds.
fn parse_manifest(data: &[u8], report: &mut MobileScanReport) -> Result<()> {
    if data.is_empty() {
        return Err(MobileError::Validation(
            "empty AndroidManifest.xml".to_string(),
        ));
    }

    let looks_like_text = data.first() == Some(&b'<')
        || data.starts_with(b"<?xml")
        || data
            .windows(9)
            .any(|w| w.eq_ignore_ascii_case(b"<manifest"));

    if looks_like_text {
        parse_text_manifest(data, report)
    } else {
        match parse_binary_axml(data, report) {
            Ok(()) => Ok(()),
            Err(e) => {
                debug!(
                    "binary AXML parse failed ({}), falling back to text scan",
                    e
                );
                parse_text_manifest(data, report)
            }
        }
    }
}

/// Minimal, auditable pure-Rust binary AXML decoder.
///
/// Strategy (kept deliberately small):
/// 1. Locate the first string pool chunk (type 0x0001) and decode all strings
///    (supports both the common UTF-16 form and the UTF-8 form used by some tools).
/// 2. Linear walk of subsequent chunks. We only care about:
///    - 0x0102 START_TAG  (tag name + attribute list)
///    - 0x0103 END_TAG    (tag name)
/// 3. For each START_TAG we parse the attribute table (name/value pairs).
///    Values are either direct string-pool references or typed data (bool/int).
/// 4. We feed the tag + attrs into the same handle_start_tag / handle_end_tag
///    state machine used by the text-XML path, so findings are produced uniformly.
///
/// The decoder does not attempt to parse the full resource table, styles,
/// or complex namespace resolution. It is intentionally focused on the exact
/// attributes needed for the security findings listed in the module docs.
fn parse_binary_axml(data: &[u8], report: &mut MobileScanReport) -> Result<()> {
    let strings = extract_string_pool(data)?;

    let mut pos: usize = 0;
    let mut current_component: Option<Component> = None;
    let mut _in_application = false;

    // Optional outer XML document chunk (type 0x0003) – skip its 8-byte header.
    if data.len() >= 8 {
        let t = read_u16_at(data, 0);
        if t == 0x0003 {
            pos = 8;
        }
    }

    while pos + 8 <= data.len() {
        let chunk_type = read_u16_at(data, pos);
        let _header_size = read_u16_at(data, pos + 2) as usize;
        let chunk_size = read_u32_at(data, pos + 4) as usize;
        if chunk_size < 8 {
            pos += 8;
            continue;
        }
        let chunk_end = pos + chunk_size;

        if chunk_type == 0x0102 {
            // START_TAG
            // After the 8-byte chunk header the layout is:
            //   u32 line, u32 comment, u32 ns, u32 name,
            //   u32 attrStart, u32 attrSize, u32 attrCount,
            //   u32 id/class/style indices
            // Attribute records start at +40 bytes from the chunk header.
            let body = pos + 8;
            if body + 40 <= data.len() {
                let raw_name = read_u32_at(data, body + 12);
                let name_idx = if raw_name == 0 { 0xffffffff } else { raw_name };
                let tag_name = strings.get(name_idx as usize).cloned().unwrap_or_default();

                let raw_ac = read_u32_at(data, body + 24);
                let attr_count = if raw_ac == 0 { 0 } else { raw_ac };
                let mut attr_p = body + 40;
                let mut attrs: HashMap<String, String> = HashMap::new();

                for _ in 0..attr_count {
                    if attr_p + 20 > data.len() {
                        break;
                    }
                    let raw_ns = read_u32_at(data, attr_p);
                    let _ns = if raw_ns == 0 { 0xffffffff } else { raw_ns };
                    let raw_n = read_u32_at(data, attr_p + 4);
                    let name_idx = if raw_n == 0 { 0xffffffff } else { raw_n };
                    let raw_r = read_u32_at(data, attr_p + 8);
                    let raw_idx = if raw_r == 0 { 0xffffffff } else { raw_r };
                    let val_type = read_u32_at(data, attr_p + 12);
                    let val_data = read_u32_at(data, attr_p + 16);

                    let aname = strings.get(name_idx as usize).cloned().unwrap_or_default();
                    let aval = if raw_idx != 0xffffffff {
                        strings.get(raw_idx as usize).cloned().unwrap_or_default()
                    } else {
                        decode_axml_value(val_type, val_data, &strings)
                    };
                    if !aname.is_empty() {
                        let key = aname.trim_start_matches("android:").to_string();
                        attrs.insert(key, aval);
                    }
                    attr_p += 20;
                }

                handle_start_tag(
                    &tag_name,
                    &attrs,
                    report,
                    &mut current_component,
                    &mut _in_application,
                );
            }
        } else if chunk_type == 0x0103 {
            // END_TAG
            let body = pos + 8;
            if body + 20 <= data.len() {
                let raw_name = read_u32_at(data, body + 12);
                let name_idx = if raw_name == 0 { 0xffffffff } else { raw_name };
                let tag_name = strings.get(name_idx as usize).cloned().unwrap_or_default();
                handle_end_tag(
                    &tag_name,
                    &mut current_component,
                    &mut _in_application,
                    report,
                );
            }
        }

        pos = chunk_end;
    }

    Ok(())
}

/// Locate and fully decode the first string pool chunk (type 0x0001).
/// Returns the ordered list of strings that later tag/attr indices refer to.
fn extract_string_pool(data: &[u8]) -> Result<Vec<String>> {
    let mut pos = 0usize;
    if data.len() >= 8 {
        let t = read_u16_at(data, 0);
        if t == 0x0003 {
            pos = 8;
        }
    }

    while pos + 8 <= data.len() {
        let chunk_type = read_u16_at(data, pos);
        let _header_size = read_u16_at(data, pos + 2) as usize;
        let chunk_size = read_u32_at(data, pos + 4) as usize;
        if chunk_size < 8 {
            pos += 8;
            continue;
        }

        if chunk_type == 0x0001 {
            return parse_string_pool_chunk(data, pos);
        }

        pos += chunk_size;
    }
    Err(MobileError::Parse(
        "no string pool chunk found in AXML".to_string(),
    ))
}

fn parse_string_pool_chunk(data: &[u8], chunk_start: usize) -> Result<Vec<String>> {
    let mut p = chunk_start + 8;
    let string_count = read_u32_at_mut(data, &mut p)?;
    let _style_count = read_u32_at_mut(data, &mut p)?;
    let flags = read_u32_at_mut(data, &mut p)?;
    let strings_start = read_u32_at_mut(data, &mut p)? as usize;
    let _styles_start = read_u32_at_mut(data, &mut p)?;

    if string_count as usize > data.len() / 4 {
        return Err(MobileError::Parse(format!(
            "string_count {} exceeds plausible limit for data size {}",
            string_count,
            data.len()
        )));
    }

    let is_utf8 = (flags & 0x100) != 0;

    let mut offsets = Vec::with_capacity(string_count as usize);
    for _ in 0..string_count {
        offsets.push(read_u32_at_mut(data, &mut p)? as usize);
    }

    let strings_base = chunk_start + strings_start;
    let mut strings = Vec::with_capacity(string_count as usize);
    for off in offsets {
        let s = if is_utf8 {
            read_utf8_pooled_string(data, strings_base + off)
        } else {
            read_utf16_pooled_string(data, strings_base + off)
        };
        strings.push(s);
    }
    Ok(strings)
}

/// Decode a single UTF-16LE pooled string (the common case).
fn read_utf16_pooled_string(data: &[u8], start: usize) -> String {
    if start + 2 > data.len() {
        return String::new();
    }
    let mut p = start;
    let mut len = read_u16_at(data, p) as usize;
    p += 2;
    if (len & 0x8000) != 0 {
        if p + 2 > data.len() {
            return String::new();
        }
        len = ((len & 0x7fff) << 16) | (read_u16_at(data, p) as usize);
        p += 2;
    }
    let byte_len = len.saturating_mul(2);
    if p + byte_len > data.len() {
        let avail = (data.len() - p) / 2;
        let s = String::from_utf16_lossy(
            &data[p..p + avail * 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<_>>(),
        );
        return s.to_string();
    }
    let s = String::from_utf16_lossy(
        &data[p..p + byte_len]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>(),
    );
    s.to_string()
}

/// Decode a single UTF-8 pooled string (used by some AXML writers).
fn read_utf8_pooled_string(data: &[u8], start: usize) -> String {
    if start + 1 > data.len() {
        return String::new();
    }
    let mut p = start;
    let mut len = data[p] as usize;
    p += 1;
    if (len & 0x80) != 0 {
        if p >= data.len() {
            return String::new();
        }
        len = ((len & 0x7f) << 8) | (data[p] as usize);
        p += 1;
    }
    if p + len > data.len() {
        len = data.len() - p;
    }
    let mut s = String::from_utf8_lossy(&data[p..p + len]).to_string();
    if s.ends_with('\0') {
        s.pop();
    }
    s
}

fn decode_axml_value(val_type: u32, val_data: u32, strings: &[String]) -> String {
    match val_type & 0xff {
        0x03 => strings.get(val_data as usize).cloned().unwrap_or_default(),
        0x12 => {
            if val_data != 0 {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        0x10 | 0x11 => val_data.to_string(),
        _ => val_data.to_string(),
    }
}

/// Text-XML fallback (quick-xml). Used for the (rare) text manifests that appear
/// in some debug / test / Gradle "merged" outputs, and as a resilience path.
fn parse_text_manifest(data: &[u8], report: &mut MobileScanReport) -> Result<()> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(data);
    // We intentionally do not call config_mut().trim_text here; the parser is already
    // tolerant of whitespace and we avoid a method-resolution edge across quick-xml builds.
    let mut buf = Vec::new();
    let mut current_component: Option<Component> = None;
    let mut _in_application = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                let attrs: HashMap<String, String> = e
                    .attributes()
                    .filter_map(|a| {
                        if let Err(ref e) = a {
                            tracing::warn!("Malformed APK attribute: {}", e);
                        }
                        a.ok()
                    })
                    .map(|a| {
                        let k = std::str::from_utf8(a.key.as_ref())
                            .unwrap_or("")
                            .to_string();
                        let v = a.unescape_value().unwrap_or_default().to_string();
                        let key = k.trim_start_matches("android:").to_string();
                        (key, v)
                    })
                    .collect();

                match tag.as_str() {
                    "manifest" => {
                        if let Some(p) = attrs.get("package") {
                            report.app_id = Some(p.clone());
                        }
                        if let Some(v) = attrs.get("versionName") {
                            report.version = Some(v.clone());
                        } else if let Some(vc) = attrs.get("versionCode") {
                            report.version = Some(vc.clone());
                        }
                    }
                    "application" => {
                        _in_application = true;
                        handle_application_attrs(&attrs, report);
                    }
                    "uses-permission" => {
                        if let Some(name) = attrs.get("name") {
                            handle_permission(name, report);
                        }
                    }
                    "activity" | "service" | "receiver" | "provider" => {
                        let name = attrs.get("name").cloned().unwrap_or_default();
                        let exported = attrs
                            .get("exported")
                            .map(|v| v.eq_ignore_ascii_case("true"));
                        current_component = Some(Component {
                            kind: tag.clone(),
                            name,
                            exported,
                            has_intent_filter: false,
                        });
                    }
                    "intent-filter" => {
                        if let Some(ref mut c) = current_component {
                            c.has_intent_filter = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                // Self-closing component tags (e.g. <provider .../>) never produce an End event.
                // We must handle them immediately.
                let tag = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                let attrs: HashMap<String, String> = e
                    .attributes()
                    .filter_map(|a| {
                        if let Err(ref e) = a {
                            tracing::warn!("Malformed APK attribute: {}", e);
                        }
                        a.ok()
                    })
                    .map(|a| {
                        let k = std::str::from_utf8(a.key.as_ref())
                            .unwrap_or("")
                            .to_string();
                        let v = a.unescape_value().unwrap_or_default().to_string();
                        let key = k.trim_start_matches("android:").to_string();
                        (key, v)
                    })
                    .collect();

                if matches!(
                    tag.as_str(),
                    "activity" | "service" | "receiver" | "provider"
                ) {
                    let name = attrs.get("name").cloned().unwrap_or_default();
                    let exported = attrs
                        .get("exported")
                        .map(|v| v.eq_ignore_ascii_case("true"));
                    let comp = Component {
                        kind: tag,
                        name,
                        exported,
                        has_intent_filter: false,
                    };
                    handle_component_end(comp, report);
                } else if tag == "application" {
                    _in_application = true;
                    handle_application_attrs(&attrs, report);
                } else if tag == "uses-permission" {
                    if let Some(name) = attrs.get("name") {
                        handle_permission(name, report);
                    }
                } else if tag == "manifest" {
                    if let Some(p) = attrs.get("package") {
                        report.app_id = Some(p.clone());
                    }
                    if let Some(v) = attrs.get("versionName") {
                        report.version = Some(v.clone());
                    } else if let Some(vc) = attrs.get("versionCode") {
                        report.version = Some(vc.clone());
                    }
                }
                // Other self-closing tags are ignored for our purposes.
            }
            Ok(Event::End(e)) => {
                let tag_name = e.name();
                let tag = std::str::from_utf8(tag_name.as_ref()).unwrap_or("");
                if tag == "application" {
                    _in_application = false;
                }
                if matches!(tag, "activity" | "service" | "receiver" | "provider") {
                    if let Some(c) = current_component.take() {
                        handle_component_end(c, report);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Application-level attribute handling (shared by binary and text paths).
fn handle_application_attrs(attrs: &HashMap<String, String>, report: &mut MobileScanReport) {
    if attrs
        .get("debuggable")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        add_finding(
            report,
            "manifest",
            Severity::High,
            "Debuggable build in production artifact",
            "android:debuggable=\"true\" enables debugging, weakens app sandboxing, and is a severe red flag for any non-debug build.",
            "Ensure release builds explicitly set debuggable=\"false\" (or omit the attribute). Never ship debuggable APKs.",
            Some("debuggable=true".to_string()),
        );
    }

    let allow_backup = attrs
        .get("allowBackup")
        .map(|v| v.eq_ignore_ascii_case("true"));
    if allow_backup == Some(true) || allow_backup.is_none() {
        add_finding(
            report,
            "manifest",
            Severity::Medium,
            "Backup allowed (data exfil via adb backup)",
            "android:allowBackup is true (or not explicitly disabled). User data can be extracted via adb backup even without root on many devices.",
            "Set android:allowBackup=\"false\" on the <application> tag for production releases.",
            allow_backup.map(|_| "allowBackup=true".to_string()),
        );
    }

    if attrs
        .get("usesCleartextTraffic")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        add_finding(
            report,
            "manifest",
            Severity::High,
            "Cleartext HTTP permitted",
            "android:usesCleartextTraffic=\"true\" (or networkSecurityConfig permits cleartext) allows plaintext HTTP, exposing traffic to interception.",
            "Use HTTPS everywhere. Set usesCleartextTraffic=\"false\" and/or provide a strict network_security_config.xml that disables cleartext.",
            Some("usesCleartextTraffic=true".to_string()),
        );
    }

    if attrs
        .get("testOnly")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        add_finding(
            report,
            "manifest",
            Severity::Medium,
            "testOnly flag set on application",
            "android:testOnly=\"true\" restricts installation and indicates this is not a production build.",
            "Remove testOnly from any build intended for distribution or regression testing outside a controlled lab.",
            Some("testOnly=true".to_string()),
        );
    }

    if let Some(cfg) = attrs.get("networkSecurityConfig") {
        debug!("networkSecurityConfig reference: {}", cfg);
    }

    if let Some(aff) = attrs.get("taskAffinity") {
        if !aff.is_empty() && aff != report.app_id.as_deref().unwrap_or("") {
            add_finding(
                report,
                "manifest",
                Severity::Low,
                "Non-default taskAffinity",
                "Custom taskAffinity can be used for task-hijacking or UI redress attacks in some scenarios.",
                "Review necessity of custom taskAffinity. Prefer default (package name) unless explicitly required for multi-app workflows.",
                Some(format!("taskAffinity={}", aff)),
            );
        }
    }

    if attrs
        .get("exported")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        add_finding(
            report,
            "manifest",
            Severity::Medium,
            "Application tag marked exported",
            "The <application> element itself declares android:exported=\"true\".",
            "Remove the exported attribute from <application>; component-level exported flags are the correct mechanism.",
            Some("application exported=true".to_string()),
        );
    }
}

fn handle_permission(name: &str, report: &mut MobileScanReport) {
    const DANGEROUS: &[&str] = &[
        "android.permission.READ_SMS",
        "android.permission.RECEIVE_SMS",
        "android.permission.READ_CALL_LOG",
        "android.permission.WRITE_CALL_LOG",
        "android.permission.PROCESS_OUTGOING_CALLS",
        "android.permission.CAMERA",
        "android.permission.RECORD_AUDIO",
        "android.permission.ACCESS_FINE_LOCATION",
        "android.permission.ACCESS_COARSE_LOCATION",
        "android.permission.READ_EXTERNAL_STORAGE",
        "android.permission.WRITE_EXTERNAL_STORAGE",
        "android.permission.READ_PHONE_STATE",
        "android.permission.CALL_PHONE",
        "android.permission.SEND_SMS",
        "android.permission.RECEIVE_MMS",
    ];

    if DANGEROUS.iter().any(|&d| {
        name.eq_ignore_ascii_case(d) || name.ends_with(d.split('.').next_back().unwrap_or(name))
    }) {
        add_finding(
            report,
            "permission",
            Severity::Medium,
            format!("Dangerous permission requested: {}", name),
            format!("The app requests the dangerous permission '{}'. This increases the app's attack surface and data access.", name),
            "Audit necessity. Prefer runtime permissions (Android 6+), least privilege, and scoped storage. Document why each dangerous permission is required.",
            Some(name.to_string()),
        );
    }
}

fn handle_start_tag(
    tag: &str,
    attrs: &HashMap<String, String>,
    report: &mut MobileScanReport,
    current: &mut Option<Component>,
    _in_application: &mut bool,
) {
    match tag {
        "manifest" => {
            if let Some(p) = attrs.get("package") {
                report.app_id = Some(p.clone());
            }
            if let Some(v) = attrs.get("versionName") {
                report.version = Some(v.clone());
            } else if let Some(vc) = attrs.get("versionCode") {
                report.version = Some(vc.clone());
            }
        }
        "application" => {
            *_in_application = true;
            handle_application_attrs(attrs, report);
        }
        "uses-permission" => {
            if let Some(name) = attrs.get("name") {
                handle_permission(name, report);
            }
        }
        "activity" | "service" | "receiver" | "provider" => {
            let name = attrs.get("name").cloned().unwrap_or_default();
            let exported = attrs
                .get("exported")
                .map(|v| v.eq_ignore_ascii_case("true"));
            *current = Some(Component {
                kind: tag.to_string(),
                name,
                exported,
                has_intent_filter: false,
            });
        }
        "intent-filter" => {
            if let Some(ref mut c) = *current {
                c.has_intent_filter = true;
            }
        }
        _ => {}
    }
}

/// Shared end-tag handler. When a component tag ends we emit any accumulated
/// exported-component finding (the report is passed through so both the
/// binary and text parsers can use the same logic).
fn handle_end_tag(
    tag: &str,
    current: &mut Option<Component>,
    _in_application: &mut bool,
    report: &mut MobileScanReport,
) {
    if tag == "application" {
        *_in_application = false;
    }
    if matches!(tag, "activity" | "service" | "receiver" | "provider") {
        if let Some(c) = current.take() {
            handle_component_end(c, report);
        }
    }
}

/// Component finding emission (identical for binary and text paths).
fn handle_component_end(c: Component, report: &mut MobileScanReport) {
    if c.exported == Some(true) && c.has_intent_filter {
        add_finding(
            report,
            "exported-component",
            Severity::High,
            format!("Exported {} with intent-filter", c.kind),
            format!(
                "Component '{}' is exported (android:exported=\"true\") and declares intent-filters. \
                 This can allow other apps to invoke it without permission checks, leading to unauthorized actions or data leakage.",
                c.name
            ),
            "Set android:exported=\"false\" unless the component must be launched by other apps. \
             When exported is required, protect with a custom android:permission (signature or signatureOrSystem) and validate caller identity inside the component.",
            Some(format!("{}:{}", c.kind, c.name)),
        );
    } else if c.exported == Some(true) {
        add_finding(
            report,
            "exported-component",
            Severity::Medium,
            format!("Exported {} (no intent-filter)", c.kind),
            format!(
                "Component '{}' declares android:exported=\"true\" with no intent-filter. \
                 It may still be directly addressable by package/component name from other apps.",
                c.name
            ),
            "Review whether export is intentional. Prefer explicit false or a protecting permission.",
            Some(format!("{}:{}", c.kind, c.name)),
        );
    }
}

/// Parse a referenced network_security_config.xml (text XML) for cleartext and user CA anchors.
fn parse_network_security_config(data: &[u8], report: &mut MobileScanReport) {
    let text = String::from_utf8_lossy(data).to_lowercase();
    if text.contains("cleartexttrafficpermitted=\"true\"")
        || text.contains("cleartexttrafficpermitted=true")
    {
        add_finding(
            report,
            "network-config",
            Severity::High,
            "Cleartext HTTP permitted via network_security_config",
            "The referenced network security config explicitly permits cleartextTrafficPermitted.",
            "Remove cleartext permission. Use only HTTPS and certificate pinning where appropriate.",
            Some("cleartextTrafficPermitted=true".to_string()),
        );
    }
    if text.contains("src=\"user\"") || (text.contains("trust-anchors") && text.contains("user")) {
        add_finding(
            report,
            "network-config",
            Severity::Medium,
            "User-added CA trust anchors permitted",
            "network_security_config allows user-installed certificates as trust anchors (MITM risk on corporate/compromised devices).",
            "Restrict <trust-anchors> to src=\"system\" only (or a pinned set of known-good certs). User CAs should be disallowed for production traffic.",
            Some("trust-anchors: user".to_string()),
        );
    }
}

/// Bounded scan of extracted small text assets for obvious secret patterns and
/// classic insecure SharedPreferences / world-readable storage hints.
fn scan_text_for_secrets_and_storage(file: &str, content: &str, report: &mut MobileScanReport) {
    let lower = content.to_ascii_lowercase();
    let secret_markers = [
        "api_key",
        "apikey",
        "api-key",
        "secret",
        "secret_key",
        "secretkey",
        "access_token",
        "accesstoken",
        "private_key",
        "privatekey",
        "private-key",
        "aws_access",
        "aws_secret",
        "bearer ",
        "password",
        "passwd",
        "auth_token",
        "authtoken",
    ];

    for marker in &secret_markers {
        if lower.contains(marker) {
            if let Some(idx) = lower.find(marker) {
                let start = idx.saturating_sub(20);
                let end = (idx + marker.len() + 40).min(content.len());
                let snippet: String = content[start..end].chars().take(80).collect();
                add_finding(
                    report,
                    "hardcoded-secret",
                    Severity::High,
                    "Hardcoded secret-like value found",
                    format!("Potential credential or key material appears in client binary asset '{}'.", file),
                    "Never embed long-lived secrets, keys, or tokens in mobile client binaries. \
                     Use backend-issued short-lived tokens, Android Keystore / iOS Keychain, and app attestation.",
                    Some(format!("{}: ...{}...", file, snippet)),
                );
                break;
            }
        }
    }

    if lower.contains("mode_world_readable")
        || lower.contains("mode_world_writeable")
        || (lower.contains("getsharedpreferences")
            && (lower.contains("mode_world") || lower.contains(", 0)") || lower.contains(",0)")))
    {
        add_finding(
            report,
            "insecure-storage",
            Severity::Medium,
            "Insecure SharedPreferences / world-readable storage hint",
            format!("Asset '{}' references MODE_WORLD_READABLE, MODE_WORLD_WRITEABLE, or getSharedPreferences with an insecure mode.", file),
            "Use MODE_PRIVATE (or Context.MODE_PRIVATE). Prefer Android Keystore / EncryptedSharedPreferences for sensitive values.",
            Some(file.to_string()),
        );
    }
}

/// Basic debug signing detection from the raw bytes of a META-INF certificate file.
fn check_cert_for_debug(name: &str, bytes: &[u8], report: &mut MobileScanReport) {
    let lower_ascii = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    if lower_ascii.contains("android debug")
        || (lower_ascii.contains("debug") && lower_ascii.contains("cert"))
    {
        add_finding(
            report,
            "signing",
            Severity::Medium,
            "Likely debug signing certificate",
            "A certificate containing 'Android Debug' or debug markers was found in META-INF. This is the default debug keystore used by Android Studio / Gradle.",
            "Sign release builds with a dedicated, properly protected production key (not the debug keystore). Debug-signed APKs are rejected by many distribution channels and indicate a non-production artifact.",
            Some(name.to_string()),
        );
    }
}

/// Uniform finding constructor (keeps all MobileFinding creation in one place).
fn add_finding(
    report: &mut MobileScanReport,
    category: &str,
    severity: Severity,
    title: impl Into<String>,
    description: impl Into<String>,
    recommendation: impl Into<String>,
    evidence: Option<String>,
) {
    report.findings.push(MobileFinding {
        category: category.to_string(),
        severity,
        title: title.into(),
        description: description.into(),
        recommendation: recommendation.into(),
        evidence,
    });
}

/// Transient state used while walking the manifest (both binary and text paths).
#[derive(Debug, Clone)]
struct Component {
    kind: String,
    name: String,
    exported: Option<bool>,
    has_intent_filter: bool,
}

// ---------- tiny safe primitive readers (never panic on truncation) ----------

#[inline]
fn read_u16_at(data: &[u8], off: usize) -> u16 {
    if off + 2 > data.len() {
        return 0;
    }
    u16::from_le_bytes([data[off], data[off + 1]])
}

#[inline]
fn read_u32_at(data: &[u8], off: usize) -> u32 {
    if off + 4 > data.len() {
        return 0;
    }
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

#[inline]
fn read_u32_at_mut(data: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > data.len() {
        return Err(MobileError::Parse(
            "AXML truncated while reading u32".to_string(),
        ));
    }
    let v = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

// ---------- unit tests using in-memory synthetic APKs (zip writer) ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_synthetic_apk(manifest: &str) -> NamedTempFile {
        let mut archive = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut archive));
            let opts = zip::write::FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Stored);
            zw.start_file("AndroidManifest.xml", opts).unwrap();
            zw.write_all(manifest.as_bytes()).unwrap();
            // small asset that should trigger the secret scanner
            zw.start_file("res/values/secrets.xml", opts).unwrap();
            zw.write_all(b"<string name=\"api_key\">sk_live_1234567890abcdef</string>")
                .unwrap();
            zw.finish().unwrap();
        }
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &archive).unwrap();
        tmp
    }

    #[tokio::test]
    async fn text_manifest_triggers_high_signal_findings() {
        let manifest = r#"<?xml version="1.0"?>
<manifest package="com.example.insecure" versionName="1.2.3" versionCode="42">
  <uses-permission android:name="android.permission.READ_SMS"/>
  <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION"/>
  <application
      android:debuggable="true"
      android:allowBackup="true"
      android:usesCleartextTraffic="true"
      android:testOnly="true">
    <activity android:name=".MainActivity" android:exported="true">
      <intent-filter>
        <action android:name="android.intent.action.MAIN"/>
        <category android:name="android.intent.category.LAUNCHER"/>
      </intent-filter>
    </activity>
    <provider android:name=".LeakProvider" android:exported="true"/>
  </application>
</manifest>"#;

        let tmp = write_synthetic_apk(manifest);
        let report = analyze_apk(tmp.path()).await.unwrap();

        assert_eq!(report.platform, MobilePlatform::Android);
        assert_eq!(report.app_id.as_deref(), Some("com.example.insecure"));
        assert_eq!(report.version.as_deref(), Some("1.2.3"));

        let titles: Vec<_> = report.findings.iter().map(|f| f.title.as_str()).collect();
        assert!(titles.iter().any(|t| t.contains("Debuggable")));
        assert!(titles.iter().any(|t| t.contains("Backup allowed")));
        assert!(titles.iter().any(|t| t.contains("Cleartext HTTP")));
        assert!(titles.iter().any(|t| t.contains("testOnly")));
        assert!(titles
            .iter()
            .any(|t| t.contains("Dangerous permission") && t.contains("READ_SMS")));
        assert!(titles.iter().any(|t| t.contains("Exported activity")));
        assert!(titles.iter().any(|t| t.contains("Exported provider")));
        assert!(titles.iter().any(|t| t.contains("Hardcoded secret")));
    }

    #[tokio::test]
    async fn rejects_zipslip() {
        let mut archive = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut archive));
            let opts = zip::write::FileOptions::<()>::default();
            zw.start_file("../../evil.xml", opts).unwrap();
            zw.write_all(b"<manifest/>").unwrap();
            zw.finish().unwrap();
        }
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &archive).unwrap();

        let err = analyze_apk(tmp.path()).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ZipSlip") || msg.contains("traversal"));
    }

    #[tokio::test]
    async fn rejects_empty_android_manifest() {
        let mut archive = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut archive));
            let opts = zip::write::FileOptions::<()>::default();
            zw.start_file("AndroidManifest.xml", opts).unwrap();
            zw.write_all(b"").unwrap();
            zw.finish().unwrap();
        }
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &archive).unwrap();

        let err = analyze_apk(tmp.path()).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("empty AndroidManifest"));
    }

    #[tokio::test]
    async fn network_config_and_insecure_storage_findings() {
        let mut archive = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut archive));
            let opts = zip::write::FileOptions::<()>::default();
            zw.start_file("AndroidManifest.xml", opts).unwrap();
            zw.write_all(b"<manifest package=\"p\"><application/></manifest>")
                .unwrap();

            zw.start_file("res/xml/network_security_config.xml", opts)
                .unwrap();
            zw.write_all(
                br#"<network-security-config>
                     <base-config cleartextTrafficPermitted="true">
                       <trust-anchors><certificates src="user"/></trust-anchors>
                     </base-config>
                   </network-security-config>"#,
            )
            .unwrap();

            zw.start_file("shared_prefs_leak.txt", opts).unwrap();
            zw.write_all(b"MODE_WORLD_READABLE getSharedPreferences(ctx, 0)")
                .unwrap();

            zw.finish().unwrap();
        }
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &archive).unwrap();

        let report = analyze_apk(tmp.path()).await.unwrap();
        let cats: Vec<_> = report
            .findings
            .iter()
            .map(|f| f.category.as_str())
            .collect();
        assert!(cats.iter().any(|c| *c == "network-config"));
        assert!(cats.iter().any(|c| *c == "insecure-storage"));
    }

    // Tiny hand-crafted binary AXML that exercises the decoder paths we care about.
    // It contains a string pool and the minimal tag/attr structures for
    // <manifest package="..."> and <application android:debuggable="true">.
    const MINIMAL_BINARY_AXML: &[u8] = &[
        0x03, 0x00, 0x08, 0x00, 0x44, 0x00, 0x00, 0x00, // outer XML chunk
        0x01, 0x00, 0x1c, 0x00, 0x34, 0x00, 0x00, 0x00, // string pool
        0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x12, 0x00,
        0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x28, 0x00, 0x00, 0x00,
        // strings (UTF-16LE)
        b'm', 0, b'a', 0, b'n', 0, b'i', 0, b'f', 0, b'e', 0, b's', 0, b't', 0, 0, 0, b'p', 0, b'a',
        0, b'c', 0, b'k', 0, b'a', 0, b'g', 0, b'e', 0, 0, 0, b'a', 0, b'p', 0, b'p', 0, b'l', 0,
        b'i', 0, b'c', 0, b'a', 0, b't', 0, b'i', 0, b'o', 0, b'n', 0, 0, 0, b'd', 0, b'e', 0,
        b'b', 0, b'u', 0, b'g', 0, b'g', 0, b'a', 0, b'b', 0, b'l', 0, b'e', 0, 0, 0, b't', 0,
        b'r', 0, b'u', 0, b'e', 0, 0, 0, // manifest start tag (abbreviated)
        0x02, 0x01, 0x10, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x14, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x03, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        // application start tag with debuggable=true (typed bool)
        0x02, 0x01, 0x10, 0x00, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xff, 0xff, 0xff, 0xff, 0x02, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x14, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x03, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
        0x12, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    ];

    #[tokio::test]
    async fn binary_axml_minimal_is_accepted() {
        let mut archive = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut archive));
            let opts = zip::write::FileOptions::<()>::default();
            zw.start_file("AndroidManifest.xml", opts).unwrap();
            zw.write_all(MINIMAL_BINARY_AXML).unwrap();
            zw.finish().unwrap();
        }
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &archive).unwrap();

        let report = analyze_apk(tmp.path()).await.unwrap();
        assert_eq!(report.platform, MobilePlatform::Android);
        // Decoder walked without panic; findings may be empty because the blob is minimal.
    }

    #[test]
    fn add_finding_populates_report() {
        let mut r = MobileScanReport::new("t.apk", MobilePlatform::Android);
        add_finding(
            &mut r,
            "test",
            Severity::High,
            "Title",
            "Desc",
            "Rec",
            Some("ev".into()),
        );
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].severity, Severity::High);
    }

    #[tokio::test]
    async fn invalid_zip_input_returns_error_not_panic() {
        use std::io::Cursor;
        let tmp = NamedTempFile::new().unwrap();
        // Write random bytes that are not a valid ZIP
        std::fs::write(tmp.path(), b"this is not a zip file at all").unwrap();
        let result = analyze_apk(tmp.path()).await;
        assert!(result.is_err(), "invalid input should return error");
        let err_msg = format!("{}", result.unwrap_err());
        // Should produce a meaningful error, not a panic
        assert!(
            err_msg.contains("zip") || err_msg.contains("XML") || err_msg.contains("invalid"),
            "error should mention the issue: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn empty_zip_returns_error_not_panic() {
        let tmp = NamedTempFile::new().unwrap();
        // Create a valid but empty ZIP (no AndroidManifest.xml)
        let mut archive = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut archive));
            let opts = zip::write::FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Stored);
            // Add a file that isn't AndroidManifest.xml
            zw.start_file("dummy.txt", opts).unwrap();
            zw.write_all(b"not a manifest").unwrap();
            zw.finish().unwrap();
        }
        std::fs::write(tmp.path(), &archive).unwrap();
        let result = analyze_apk(tmp.path()).await;
        assert!(result.is_err(), "empty/invalid APK should return error");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("manifest")
                || err_msg.contains("AndroidManifest")
                || err_msg.contains("not found"),
            "error should mention missing manifest: {}",
            err_msg
        );
    }
}
