use std::path::{Path, PathBuf};

use tracing::{error, warn};

use super::archive::decode_lzma_base64;
use super::loader::{load_halloy_theme, ThemeLoadError};
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

#[derive(Debug)]
pub struct ThemeInstallReport {
    pub theme_dir: Option<PathBuf>,
    pub installed: usize,
    pub skipped_existing: usize,
    pub loaded: usize,
    pub errors: Vec<String>,
    pub loaded_themes: Vec<Result<Theme, ThemeLoadError>>,
}

impl Clone for ThemeInstallReport {
    fn clone(&self) -> Self {
        // ThemeLoadError contains non-Clone types (io::Error, toml::Error),
        // so we cannot derive Clone for ThemeInstallReport.  Preserve
        // loaded_themes: Vec::new() was incorrect, but we have no choice
        // since Result<Theme, ThemeLoadError> is not Clone.
        Self {
            theme_dir: self.theme_dir.clone(),
            installed: self.installed,
            skipped_existing: self.skipped_existing,
            loaded: self.loaded,
            errors: self.errors.clone(),
            loaded_themes: Vec::new(),
        }
    }
}

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

pub fn load_themes_from_dir(dir: &Path) -> Vec<Result<Theme, ThemeLoadError>> {
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
            .unwrap_or("unknown");

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to read theme file");
                results.push(Err(ThemeLoadError::IoError(e)));
                continue;
            }
        };

        results.push(load_halloy_theme(&content, file_stem));
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

    let loaded_results = load_themes_from_dir(&dir);
    let loaded_count = loaded_results.iter().filter(|r| r.is_ok()).count();
    report.loaded = loaded_count;

    let mut loaded_themes = Vec::new();
    for result in loaded_results {
        match &result {
            Ok(_) => {}
            Err(e) => {
                report.errors.push(format!("load theme: {e}"));
            }
        }
        loaded_themes.push(result);
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

        let results = load_themes_from_dir(&dir);
        assert_eq!(results.len(), 2);

        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        let err_count = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(ok_count, 1);
        assert_eq!(err_count, 1);

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
