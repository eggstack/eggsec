use std::sync::Mutex;

static CLIPBOARD: std::sync::LazyLock<Mutex<Option<arboard::Clipboard>>> =
    std::sync::LazyLock::new(|| Mutex::new(arboard::Clipboard::new().ok()));

pub struct Clipboard;

impl Clipboard {
    pub fn get() -> Option<String> {
        let mut guard = match CLIPBOARD.lock() {
            Ok(g) => g,
            Err(_) => return None,
        };
        match guard.as_mut() {
            Some(cb) => cb.get_text().ok(),
            None => None,
        }
    }

    pub fn set(text: &str) -> bool {
        let mut guard = match CLIPBOARD.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        match guard.as_mut() {
            Some(cb) => cb.set_text(text).is_ok(),
            None => false,
        }
    }

    pub fn clear() -> bool {
        Self::set("")
    }

    pub fn is_available() -> bool {
        match CLIPBOARD.lock() {
            Ok(guard) => guard.is_some(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_availability() {
        let available = Clipboard::is_available();
        tracing::debug!("Clipboard available: {}", available);
    }

    #[test]
    fn test_clipboard_set_get() {
        if !Clipboard::is_available() {
            return;
        }
        let test_text = "Slapper clipboard test";
        assert!(Clipboard::set(test_text));
        assert_eq!(Clipboard::get(), Some(test_text.to_string()));
        Clipboard::clear();
    }
}
