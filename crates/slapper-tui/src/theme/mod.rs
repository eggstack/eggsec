pub mod archive;
pub mod builtin;
pub mod install;
pub mod legacy;
pub mod loader;
pub mod manager;
pub mod packaged;
pub mod palette;
pub mod style;

pub fn canonical_theme_id(name: &str) -> String {
    let trimmed = name.trim().trim_end_matches(".toml");
    let mut canonical = String::new();
    let mut last_was_separator = false;

    for ch in trimmed.chars() {
        if matches!(ch, ' ' | '_' | '-') {
            if !canonical.is_empty() && !last_was_separator {
                canonical.push('-');
                last_was_separator = true;
            }
            continue;
        }

        canonical.extend(ch.to_lowercase());
        last_was_separator = false;
    }

    canonical.trim_matches('-').to_string()
}

pub fn display_theme_name(id: &str) -> String {
    id.trim()
        .trim_end_matches(".toml")
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub use legacy::sync_theme_to_thread_local;
pub use manager::ThemeManager;
pub use palette::Theme;
#[allow(unused_imports)]
pub use palette::ThemeMode;
