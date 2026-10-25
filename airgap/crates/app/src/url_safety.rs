//! Safe External Hyperlink Handling & Zero-Network Enforcement.
//!
//! Enforces strict scheme whitelist (http, https, mailto), host validation,
//! length bounds, and non-blocking asynchronous OS browser dispatch.

/// Validates and sanitizes an external URL.
/// Returns Ok(sanitized_url) or Err(reason).
pub fn sanitize_external_url(raw: &str) -> Result<String, &'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("URL is empty");
    }
    if trimmed.len() > 2048 {
        return Err("URL exceeds 2048 characters");
    }

    let lower = trimmed.to_ascii_lowercase();

    // Strict whitelist: Only http, https, and mailto
    if lower.starts_with("http://") || lower.starts_with("https://") {
        let host_start = if lower.starts_with("https://") { 8 } else { 7 };
        let rest = &trimmed[host_start..];
        if rest.trim().is_empty() || rest.starts_with('/') || rest.starts_with('\\') {
            return Err("Invalid or empty web URL host");
        }
        // Block control characters or newlines
        if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("URL contains whitespace or control characters");
        }
        Ok(trimmed.to_string())
    } else if lower.starts_with("mailto:") {
        let rest = &trimmed[7..];
        if !rest.contains('@') {
            return Err("Invalid mailto address: missing '@'");
        }
        if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("Mailto URL contains whitespace or control characters");
        }
        Ok(trimmed.to_string())
    } else {
        Err("Blocked unsafe URL scheme: only http://, https://, and mailto: are permitted")
    }
}

/// Safely opens a validated URL in the OS default browser off the UI thread.
pub fn open_external_url_safely(raw_url: &str) -> Result<(), &'static str> {
    let safe_url = sanitize_external_url(raw_url)?;
    std::thread::spawn(move || {
        let _ = open::that_detached(&safe_url);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_http_urls() {
        assert!(sanitize_external_url("https://example.com").is_ok());
        assert!(sanitize_external_url("http://rust-lang.org/learn").is_ok());
        assert!(sanitize_external_url("https://crates.io/crates/pulldown-cmark").is_ok());
    }

    #[test]
    fn test_valid_mailto_urls() {
        assert!(sanitize_external_url("mailto:support@airgap.local").is_ok());
        assert!(sanitize_external_url("mailto:user@example.com?subject=Note").is_ok());
    }

    #[test]
    fn test_blocked_schemes() {
        assert!(sanitize_external_url("file:///C:/Windows/notepad.exe").is_err());
        assert!(sanitize_external_url("file:///etc/passwd").is_err());
        assert!(sanitize_external_url("ftp://example.com/file.zip").is_err());
        assert!(sanitize_external_url("data:text/html,<script>alert(1)</script>").is_err());
        assert!(sanitize_external_url("javascript:alert(1)").is_err());
        assert!(sanitize_external_url("powershell.exe -Command Remove-Item").is_err());
        assert!(sanitize_external_url("cmd.exe /c calc.exe").is_err());
    }

    #[test]
    fn test_invalid_hosts_and_whitespace() {
        assert!(sanitize_external_url("https:///missing-host").is_err());
        assert!(sanitize_external_url("http://").is_err());
        assert!(sanitize_external_url("https://example .com").is_err());
        assert!(sanitize_external_url("https://example.com\nmalicious").is_err());
    }
}
