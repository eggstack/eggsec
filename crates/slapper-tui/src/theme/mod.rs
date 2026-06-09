pub mod archive;
pub mod builtin;
pub mod install;
pub mod legacy;
pub mod loader;
pub mod manager;
pub mod packaged;
pub mod palette;
pub mod style;

pub use legacy::sync_theme_to_thread_local;
pub use manager::ThemeManager;
pub use palette::{Theme, ThemeMode};
