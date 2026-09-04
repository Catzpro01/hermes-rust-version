/// Redacts common credential-like values from text at the output boundary.
pub fn redact_credentials(text: &str) -> String {
    let mut result = text.to_owned();
    for prefix in ["sk-proj-", "sk-"] {
        result = redact_pattern(&result, prefix, 20);
    }
    for key in ["API_KEY=", "api_key=", "SECRET="] {
        result = redact_kv(&result, key);
    }
    redact_bearer(&result)
}
fn redact_pattern(text: &str, prefix: &str, min_suffix: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(prefix) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + prefix.len()..];
        let len = after
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(after.len());
        out.push_str(prefix);
        if len >= min_suffix {
            out.push_str("***REDACTED***");
        } else {
            out.push_str(&after[..len]);
        }
        rest = &after[len..];
    }
    out.push_str(rest);
    out
}
fn redact_kv(text: &str, key: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(key) {
        out.push_str(&rest[..pos + key.len()]);
        let after = &rest[pos + key.len()..];
        let len = after
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(after.len());
        if len > 4 {
            out.push_str("***REDACTED***");
        } else {
            out.push_str(&after[..len]);
        }
        rest = &after[len..];
    }
    out.push_str(rest);
    out
}
fn redact_bearer(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("Bearer ") {
        out.push_str(&rest[..pos + 7]);
        let after = &rest[pos + 7..];
        let len = after.find(char::is_whitespace).unwrap_or(after.len());
        if len > 10 {
            out.push_str("***REDACTED***");
        } else {
            out.push_str(&after[..len]);
        }
        rest = &after[len..];
    }
    out.push_str(rest);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_openai_key() {
        let x = redact_credentials("use sk-proj-abc123def456ghi789jkl012mno345pqr");
        assert!(!x.contains("abc123def456"));
        assert!(x.contains("***REDACTED***"));
    }
    #[test]
    fn redacts_kv() {
        let x = redact_credentials("API_KEY=super-secret-search-fixture here");
        assert!(!x.contains("super-secret"));
    }
    #[test]
    fn redacts_bearer() {
        let x = redact_credentials("Bearer eyJhbGciOiJIUzI1NiIsr");
        assert!(!x.contains("eyJhbGci"));
    }
    #[test]
    fn preserves_safe() {
        let x = "deploy safely";
        assert_eq!(redact_credentials(x), x);
    }
}
