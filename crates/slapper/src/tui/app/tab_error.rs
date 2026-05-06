use std::fmt;

#[derive(Debug, Clone)]
pub enum TabError {
    Network(String),
    Auth(String),
    Config(String),
    Resource(String),
    Target(String),
    Internal(String),
    Unknown(String),
}

impl TabError {
    pub fn message(&self) -> String {
        match self {
            TabError::Network(msg) => msg.clone(),
            TabError::Auth(msg) => msg.clone(),
            TabError::Config(msg) => msg.clone(),
            TabError::Resource(msg) => msg.clone(),
            TabError::Target(msg) => msg.clone(),
            TabError::Internal(msg) => msg.clone(),
            TabError::Unknown(msg) => msg.clone(),
        }
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            TabError::Network(_) | TabError::Auth(_) | TabError::Resource(_)
        )
    }
}

impl fmt::Display for TabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl From<String> for TabError {
    fn from(s: String) -> Self {
        TabError::Unknown(s)
    }
}

impl From<&str> for TabError {
    fn from(s: &str) -> Self {
        TabError::Unknown(s.to_string())
    }
}
