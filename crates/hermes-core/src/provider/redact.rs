use crate::config::SecretString;

/// Removes a provider credential from a diagnostic message.
pub fn redact(message: &str, key: &SecretString) -> String {
    if key.expose().is_empty() {
        message.to_owned()
    } else {
        message.replace(key.expose(), "***REDACTED***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_secret() {
        let key = SecretString::from("secret");
        assert_eq!(redact("bad secret value", &key), "bad ***REDACTED*** value");
        assert!(!redact("bad secret value", &key).contains("secret"));
    }
}
