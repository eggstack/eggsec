use std::cell::RefCell;

use super::manager::ThemeManager;
use super::palette::Theme;

thread_local! {
    pub static THEME_MANAGER: RefCell<ThemeManager> = RefCell::new(ThemeManager::new());
}

pub fn sync_theme_to_thread_local(theme: &Theme) {
    THEME_MANAGER.with(|tm| {
        tm.borrow_mut().set_current_for_legacy_sync(theme);
    });
}

pub fn current_theme() -> Theme {
    THEME_MANAGER.with(|tm| tm.borrow().current().clone())
}

#[macro_export]
macro_rules! tc {
    ($field:ident) => {
        $crate::theme::legacy::THEME_MANAGER.with(|tm| tm.borrow().current().colors.$field)
    };
}
