pub(crate) fn make_friendly_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("connection refused") {
        return "Connection refused. The target may be down or not accepting connections."
            .to_string();
    }
    if error_str.contains("timeout") || error_str.contains("timed out") {
        return "Request timed out. The target may be slow or unreachable.".to_string();
    }
    if error_str.contains("name or service not known") || error_str.contains("dns") {
        return "DNS resolution failed. Please check the target domain is correct.".to_string();
    }
    if error_str.contains("certificate") || error_str.contains("tls") || error_str.contains("ssl") {
        return "SSL/TLS error. The website may have certificate issues.".to_string();
    }
    if error_str.contains("permission denied") {
        return "Permission denied. Try running with elevated privileges.".to_string();
    }
    if error_str.contains("rate limit") || error_str.contains("429") {
        return "Rate limited. Too many requests. Please try again later.".to_string();
    }
    if error_str.contains("unauthorized")
        || error_str.contains("401")
        || error_str.contains("forbidden")
    {
        return "Unauthorized. Check your API keys in the configuration.".to_string();
    }
    if error_str.contains("not found") || error_str.contains("404") {
        return "Resource not found. The target may not exist.".to_string();
    }
    if error_str.contains("no route to host") || error_str.contains("network") {
        return "Network error. Check your internet connection.".to_string();
    }
    if error_str.contains("broken pipe") || error_str.contains("reset") {
        return "Connection broken. The remote host closed the connection.".to_string();
    }

    format!("Error: {}", error)
}
