use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;
use tracing::{error, warn};

use super::archive::decode_lzma_base64;
use super::loader::{load_halloy_theme, ThemeLoadError};
use super::manager::ThemeSource;
use super::packaged::{PACKAGED_THEMES_LZMA_BASE64, PACKAGED_THEMES_VERSION};
use super::palette::Theme;

/// Marker filename used to short-circuit LZMA decoding on subsequent launches
/// when no packaged theme has changed since the previous run.
const VERSION_MARKER_FILENAME: &str = ".eggsec-packaged-themes-version";

#[derive(Debug, thiserror::Error)]
pub enum ThemeInstallError {
    #[error("failed to decode packaged themes: {0}")]
    ArchiveError(#[from] super::archive::ThemeArchiveError),
}

/// A single loaded theme record with metadata for correct source attribution.
#[derive(Debug)]
pub struct LoadedThemeRecord {
    /// The parsed theme (or error).
    pub result: Result<Theme, ThemeLoadError>,
    /// File stem (without .toml extension).
    pub file_stem: String,
    /// Inferred source based on whether the file stem is in the packaged set.
    pub source: ThemeSource,
    /// Contrast warnings produced during loading (empty if none).
    pub contrast_warnings: Vec<String>,
}

#[derive(Debug)]
pub struct ThemeInstallReport {
    pub theme_dir: Option<PathBuf>,
    pub installed: usize,
    pub skipped_existing: usize,
    pub loaded: usize,
    pub errors: Vec<String>,
    pub loaded_themes: Vec<LoadedThemeRecord>,
}

// ThemeLoadError contains non-Clone types (io::Error, toml::Error),
// so ThemeInstallReport does not implement Clone. It is consumed
// via channels and does not need to be cloned.

pub fn user_theme_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "eggsec").map(|proj| {
        let base = if cfg!(target_os = "windows") {
            proj.data_dir()
        } else {
            proj.config_dir()
        };
        base.join("themes")
    })
}

pub fn decode_packaged_themes() -> Result<Vec<super::archive::PackagedThemeFile>, ThemeInstallError>
{
    Ok(decode_lzma_base64(PACKAGED_THEMES_LZMA_BASE64)?)
}

fn is_safe_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

fn atomic_write(dest: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    let parent = dest.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent directory")
    })?;
    let tmp = parent.join(format!(
        ".{}.{}.tmp",
        dest.file_name().and_then(|n| n.to_str()).unwrap_or("theme"),
        std::process::id()
    ));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

pub fn ensure_packaged_themes_installed(dir: &Path) -> ThemeInstallReport {
    let mut report = ThemeInstallReport {
        theme_dir: Some(dir.to_path_buf()),
        installed: 0,
        skipped_existing: 0,
        loaded: 0,
        errors: Vec::new(),
        loaded_themes: Vec::new(),
    };

    if let Err(e) = std::fs::create_dir_all(dir) {
        error!(error = %e, "failed to create themes directory");
        report.errors.push(format!("create_dir_all: {e}"));
        return report;
    }

    // Short-circuit: if the version marker matches, every packaged theme was
    // installed on a previous run and we can skip the LZMA decode + per-file
    // existence checks. ~9KB of base64 + 50+ stat() calls adds measurable
    // startup latency for a no-op.
    let version_marker = dir.join(VERSION_MARKER_FILENAME);
    if let Ok(marker_contents) = std::fs::read_to_string(&version_marker) {
        if marker_contents.trim() == PACKAGED_THEMES_VERSION.to_string() {
            return report;
        }
    }

    let packaged = match decode_packaged_themes() {
        Ok(files) => files,
        Err(e) => {
            error!(error = %e, "failed to decode packaged themes");
            report.errors.push(format!("decode: {e}"));
            return report;
        }
    };

    for file in packaged {
        let dest = dir.join(&file.relative_path);

        if !is_safe_path(&file.relative_path) {
            warn!(path = %file.relative_path.display(), "skipping unsafe path");
            report
                .errors
                .push(format!("unsafe path: {}", file.relative_path.display()));
            continue;
        }

        if dest.exists() {
            report.skipped_existing += 1;
            continue;
        }

        match atomic_write(&dest, &file.content) {
            Ok(()) => report.installed += 1,
            Err(e) => {
                warn!(path = %dest.display(), error = %e, "failed to write theme file");
                report.errors.push(format!("write {}: {e}", dest.display()));
            }
        }
    }

    if report.errors.is_empty() {
        if let Err(e) = std::fs::write(&version_marker, PACKAGED_THEMES_VERSION.to_string()) {
            warn!(error = %e, "failed to write theme version marker");
        }
    }

    report
}

pub fn load_themes_from_dir(
    dir: &Path,
    packaged_ids: &FxHashSet<String>,
) -> Vec<LoadedThemeRecord> {
    let mut results = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(error = %e, "failed to read themes directory");
            return results;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let source = if packaged_ids.contains(&file_stem) {
            ThemeSource::Packaged
        } else {
            ThemeSource::Custom
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to read theme file");
                results.push(LoadedThemeRecord {
                    result: Err(ThemeLoadError::IoError(e)),
                    file_stem,
                    source,
                    contrast_warnings: Vec::new(),
                });
                continue;
            }
        };

        let result = load_halloy_theme(&content, &file_stem);
        let contrast_warnings = match &result {
            Ok(theme) => {
                // Collect contrast warnings for the loaded theme.
                let mut warnings = Vec::new();
                use super::contrast::{check_contrast, contrast_ratio};
                let pairs = [
                    ("text", "background", theme.colors.text, theme.colors.background),
                    ("selected_text", "selected", theme.colors.selected_text, theme.colors.selected),
                ];
                for (fg_name, bg_name, fg, bg) in pairs {
                    if matches!(fg, ratatui::style::Color::Rgb(..))
                        && matches!(bg, ratatui::style::Color::Rgb(..))
                        && !check_contrast(fg, bg, 4.5)
                    {
                        let ratio = contrast_ratio(fg, bg);
                        warnings.push(format!(
                            "{fg_name}/{bg_name} contrast ratio {:.2}:1 is below 4.5:1 minimum",
                            ratio,
                        ));
                    }
                }
                warnings
            }
            Err(_) => Vec::new(),
        };

        results.push(LoadedThemeRecord {
            result,
            file_stem,
            source,
            contrast_warnings,
        });
    }

    results
}

pub fn load_and_install_themes() -> ThemeInstallReport {
    let dir = match user_theme_dir() {
        Some(d) => d,
        None => {
            return ThemeInstallReport {
                theme_dir: None,
                installed: 0,
                skipped_existing: 0,
                loaded: 0,
                errors: vec!["could not determine user theme directory".to_string()],
                loaded_themes: Vec::new(),
            };
        }
    };

    let mut report = ensure_packaged_themes_installed(&dir);

    // Build set of packaged theme IDs for correct source attribution.
    // This avoids the fragile "count installed" heuristic that breaks when
    // packaged themes already exist on disk (installed == 0).
    let packaged_ids = match decode_packaged_themes() {
        Ok(files) => {
            let mut ids = FxHashSet::default();
            for file in &files {
                if let Some(stem) = file.relative_path.file_stem().and_then(|s| s.to_str()) {
                    ids.insert(stem.to_string());
                }
            }
            ids
        }
        Err(_) => FxHashSet::default(),
    };

    let loaded_results = load_themes_from_dir(&dir, &packaged_ids);
    let loaded_count = loaded_results.iter().filter(|r| r.result.is_ok()).count();
    report.loaded = loaded_count;

    let mut loaded_themes = Vec::new();
    for record in loaded_results {
        if let Err(e) = &record.result {
            report.errors.push(format!("load theme: {e}"));
        }
        loaded_themes.push(record);
    }
    report.loaded_themes = loaded_themes;

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_theme_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("eggsec_theme_test_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn installer_skips_existing_files() {
        let dir = temp_theme_dir("skip_existing");
        fs::create_dir_all(&dir).unwrap();

        let content = "[general]\nbackground = \"#2E3440\"";
        let dest = dir.join("ExistingTheme.toml");
        fs::write(&dest, content).unwrap();

        let _report = ensure_packaged_themes_installed(&dir);
        // File should be unchanged regardless of packaged data decode result
        assert_eq!(fs::read_to_string(&dest).unwrap(), content);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn installer_creates_directory_if_missing() {
        let dir = temp_theme_dir("create_dir");
        assert!(!dir.exists());

        let _report = ensure_packaged_themes_installed(&dir);
        assert!(dir.exists());
        // Should not panic; errors are collected, not fatal

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        assert!(!is_safe_path(Path::new("/etc/passwd")));
        assert!(!is_safe_path(Path::new("../escape")));
        assert!(!is_safe_path(Path::new("foo/../../bar")));
        assert!(is_safe_path(Path::new("Nord.toml")));
        assert!(is_safe_path(Path::new("sub/dir/theme.toml")));
    }

    #[test]
    fn load_themes_from_dir_returns_results() {
        let dir = temp_theme_dir("load_dir");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("valid.toml"),
            "[general]\nbackground = \"#000000\"",
        )
        .unwrap();
        fs::write(dir.join("invalid.toml"), "{{bad toml").unwrap();
        fs::write(dir.join("not_a_theme.txt"), "ignored").unwrap();

        let packaged_ids = FxHashSet::default();
        let results = load_themes_from_dir(&dir, &packaged_ids);
        assert_eq!(results.len(), 2);

        let ok_count = results.iter().filter(|r| r.result.is_ok()).count();
        let err_count = results.iter().filter(|r| r.result.is_err()).count();
        assert_eq!(ok_count, 1);
        assert_eq!(err_count, 1);

        // Verify source attribution: neither is packaged
        for record in &results {
            assert_eq!(record.source, ThemeSource::Custom);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_themes_from_dir_attributes_packaged_sources() {
        let dir = temp_theme_dir("packaged_source");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("packaged_theme.toml"),
            "[general]\nbackground = \"#000000\"",
        )
        .unwrap();
        fs::write(
            dir.join("custom_theme.toml"),
            "[general]\nbackground = \"#FFFFFF\"",
        )
        .unwrap();

        let mut packaged_ids = FxHashSet::default();
        packaged_ids.insert("packaged_theme".to_string());

        let results = load_themes_from_dir(&dir, &packaged_ids);
        assert_eq!(results.len(), 2);

        let packaged = results.iter().find(|r| r.file_stem == "packaged_theme").unwrap();
        assert_eq!(packaged.source, ThemeSource::Packaged);

        let custom = results.iter().find(|r| r.file_stem == "custom_theme").unwrap();
        assert_eq!(custom.source, ThemeSource::Custom);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_theme_dir_returns_some() {
        let dir = user_theme_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.ends_with("themes"));
    }
}
