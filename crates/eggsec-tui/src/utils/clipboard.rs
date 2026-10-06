use parking_lot::Mutex;

/// Clipboard handle plus the init failure, so a headless / SSH / display-less
/// host can tell the user *why* copy and paste are unavailable instead of
/// silently doing nothing.
struct ClipboardSlot {
    handle: Option<arboard::Clipboard>,
    init_error: Option<String>,
}

static CLIPBOARD: std::sync::LazyLock<Mutex<ClipboardSlot>> = std::sync::LazyLock::new(|| {
    match arboard::Clipboard::new() {
        Ok(handle) => Mutex::new(ClipboardSlot {
            handle: Some(handle),
            init_error: None,
        }),
        Err(e) => {
            // The first copy/paste attempt on such a host would otherwise be a
            // silent no-op; keep the reason for the notification overlay.
            let reason = e.to_string();
            tracing::warn!("Clipboard unavailable: {}", reason);
            Mutex::new(ClipboardSlot {
                handle: None,
                init_error: Some(reason),
            })
        }
    }
});

pub struct Clipboard;

impl Clipboard {
    pub fn get() -> Option<String> {
        let mut guard = CLIPBOARD.lock();
        match guard.handle.as_mut() {
            Some(cb) => match cb.get_text() {
                Ok(text) => Some(text),
                // An empty clipboard is the common case, not a failure.
                Err(e) => {
                    tracing::debug!("Clipboard read failed: {}", e);
                    None
                }
            },
            None => {
                tracing::debug!("Clipboard read skipped: no clipboard available");
                None
            }
        }
    }

    pub fn set(text: &str) -> bool {
        let mut guard = CLIPBOARD.lock();
        match guard.handle.as_mut() {
            Some(cb) => match cb.set_text(text) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!("Clipboard write failed: {}", e);
                    false
                }
            },
            None => {
                tracing::warn!("Clipboard write skipped: no clipboard available");
                false
            }
        }
    }

    pub fn clear() -> bool {
        Self::set("")
    }

    pub fn is_available() -> bool {
        CLIPBOARD.lock().handle.is_some()
    }

    /// User-facing message explaining why a clipboard action cannot run, or
    /// `None` when the clipboard works.
    ///
    /// Callers surface this through the notification overlay
    /// (`NotificationSeverity::Error`) so pressing `y` on a headless / SSH host
    /// reports the failure instead of failing silently.
    pub fn unavailable_message(action: &str) -> Option<String> {
        CLIPBOARD.lock().init_error.as_ref().map(|reason| {
            format!(
                "Clipboard {} failed: {} (no clipboard available on this display)",
                action, reason
            )
        })
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
        let test_text = "Eggsec clipboard test";
        assert!(Clipboard::set(test_text));
        assert_eq!(Clipboard::get(), Some(test_text.to_string()));
        Clipboard::clear();
    }

    #[test]
    fn test_unavailable_message_matches_availability() {
        // The message exists exactly when the clipboard is unusable, so a caller
        // can raise a notification without probing the clipboard twice.
        assert_eq!(
            Clipboard::unavailable_message("copy").is_some(),
            !Clipboard::is_available()
        );
    }

    #[test]
    fn test_unavailable_message_names_the_failed_action() {
        if let Some(msg) = Clipboard::unavailable_message("paste") {
            assert!(msg.contains("paste"), "message must name the action");
        }
    }
}
