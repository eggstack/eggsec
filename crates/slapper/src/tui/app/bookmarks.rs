use crate::tui::tabs::Tab;
use rustc_hash::FxHashSet;

pub fn toggle_bookmark(bookmarks: &mut FxHashSet<String>, tab: Tab) {
    let id = tab.stable_id().to_string();
    if bookmarks.contains(&id) {
        bookmarks.remove(&id);
    } else {
        bookmarks.insert(id);
    }
}

pub fn is_bookmarked(bookmarks: &FxHashSet<String>, tab: Tab) -> bool {
    bookmarks.contains(tab.stable_id())
}

pub fn get_bookmarked_tab_ids(bookmarks: &FxHashSet<String>) -> Vec<String> {
    bookmarks.iter().cloned().collect()
}
