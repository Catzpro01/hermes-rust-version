/// Redacts common credential-like values from text at the output boundary.
pub fn redact_credentials(text: &str) -> String {
    let mut result = text.to_owned();
    result = redact_pattern(&result, "sk-proj-", 12);
    result = redact_pattern(&result, "sk-", 8);
    for key in ["API_KEY=", "api_key=", "SECRET="] {
        result = redact_kv(&result, key);
    }
    redact_bearer(&result)
}
fn redact_pattern(text: &str, prefix: &str, min_suffix: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(prefix) {
        // A secret prefix only counts at a token boundary. Without this check
        // `find` matches the substring inside ordinary words, so lowering
        // `min_suffix` redacts things like "ask-anything" or "desk-setup".
        let boundary = rest[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
        if !boundary {
            // Emit the matched text verbatim and keep scanning past it.
            out.push_str(&rest[..pos + prefix.len()]);
            rest = &rest[pos + prefix.len()..];
            continue;
        }
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
    #[test]
    fn redacts_short_sk_key() {
        assert!(!redact_credentials("key sk-short123").contains("sk-short123"));
    }
    #[test]
    fn preserves_non_key_sk_words() {
        // Ordinary words that merely contain "sk-" must survive redaction.
        for word in [
            "ask-anything",
            "desk-setup",
            "risk-assessment",
            "task-tracking",
            "ask-for-help",
        ] {
            assert_eq!(
                redact_credentials(word),
                word,
                "false positive on ordinary word {word:?}"
            );
        }
    }
    #[test]
    fn redacts_sk_at_start_of_text() {
        assert!(!redact_credentials("sk-short123 leaked").contains("sk-short123"));
    }
    #[test]
    fn redacts_short_sk_after_punctuation() {
        let x = redact_credentials("token=sk-abc12345;");
        assert!(!x.contains("abc12345"), "leaked: {x}");
    }
    #[test]
    fn preserves_bare_sk_prefix() {
        assert_eq!(redact_credentials("the sk- prefix"), "the sk- prefix");
    }
}
