//! Utility functions and helpers

pub mod time;
pub mod syscalls;

use anyhow::Result;

/// Convert bytes to a hexadecimal string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse a duration string (e.g., "30s", "5m", "1h")
pub fn parse_duration(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();

    if let Some(num_str) = s.strip_suffix('s') {
        let secs: u64 = num_str.parse()?;
        Ok(std::time::Duration::from_secs(secs))
    } else if let Some(num_str) = s.strip_suffix('m') {
        let mins: u64 = num_str.parse()?;
        Ok(std::time::Duration::from_secs(mins * 60))
    } else if let Some(num_str) = s.strip_suffix('h') {
        let hours: u64 = num_str.parse()?;
        Ok(std::time::Duration::from_secs(hours * 3600))
    } else {
        // Default to seconds if no suffix
        let secs: u64 = s.parse()?;
        Ok(std::time::Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap().as_secs(), 30);
        assert_eq!(parse_duration("5m").unwrap().as_secs(), 300);
        assert_eq!(parse_duration("1h").unwrap().as_secs(), 3600);
        assert_eq!(parse_duration("60").unwrap().as_secs(), 60);
    }

    #[test]
    fn test_bytes_to_hex() {
        assert_eq!(bytes_to_hex(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }
}
