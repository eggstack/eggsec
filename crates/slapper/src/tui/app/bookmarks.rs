use crate::tui::tabs::Tab;

pub fn toggle_bookmark(bookmarks: &mut std::collections::HashSet<String>, tab: Tab) {
    let id = tab.stable_id().to_string();
    if bookmarks.contains(&id) {
        bookmarks.remove(&id);
    } else {
        bookmarks.insert(id);
    }
}

pub fn is_bookmarked(bookmarks: &std::collections::HashSet<String>, tab: Tab) -> bool {
    bookmarks.contains(tab.stable_id())
}

pub fn get_bookmarked_tab_ids(bookmarks: &std::collections::HashSet<String>) -> Vec<String> {
    bookmarks.iter().cloned().collect()
}