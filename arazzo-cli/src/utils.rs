pub fn redact_url_password(url: &str) -> String {
    // Simple redaction: replace password in postgres://user:pass@host format
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(scheme_end) = url.find("://") {
                let scheme = &url[..scheme_end + 3];
                let user = &url[scheme_end + 3..colon_pos];
                let rest = &url[at_pos..];
                return format!("{}{}:***{}", scheme, user, rest);
            }
        }
    }
    url.to_string()
}
