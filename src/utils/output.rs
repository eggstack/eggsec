#![allow(dead_code)]

use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    println!("{}", json);
    Ok(())
}

pub fn print_json_compact<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string(value)?;
    println!("{}", json);
    Ok(())
}

pub fn print_success(message: &str) {
    println!("[✓] {}", message);
}

pub fn print_error(message: &str) {
    eprintln!("[✗] {}", message);
}

pub fn print_warning(message: &str) {
    println!("[!] {}", message);
}

pub fn print_info(message: &str) {
    println!("[*] {}", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_json() {
        let data = serde_json::json!({"key": "value"});
        let result = print_json(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_success() {
        print_success("Test passed");
    }

    #[test]
    fn test_print_error() {
        print_error("Test failed");
    }
}
