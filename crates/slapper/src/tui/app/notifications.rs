#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSeverity {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Notification {
    pub message: String,
    pub severity: NotificationSeverity,
    pub created_at: std::time::Instant,
    pub timeout_secs: u64,
}

impl Notification {
    pub fn new(message: String, severity: NotificationSeverity) -> Self {
        Self {
            message,
            severity,
            created_at: std::time::Instant::now(),
            timeout_secs: 5,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > self.timeout_secs
    }
}