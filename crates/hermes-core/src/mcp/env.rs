//! Spec 011b (Ticket 03) — environment variable expansion for MCP server `env`.
//!
//! A config value may contain `${VAR_NAME}` (or `$VAR_NAME`) placeholders that
//! are expanded from the process environment at spawn time. A placeholder that
//! references an unset variable is an error that names the variable (never its
//! value). Expansion is done in memory; the expanded values are not logged.

use super::error::McpError;

/// Expands `$NAME` and `${NAME}` placeholders in `value` from `lookup`.
/// `lookup(name)` returns `Some` if set. Unset references produce an error.
/// Returns the expanded string.
pub fn expand_env_value<F>(value: &str, lookup: &F) -> Result<String, McpError>
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            // Try braced ${NAME}
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                if let Some(close) = value[i + 2..].find('}') {
                    let name = &value[i + 2..i + 2 + close];
                    let name = name.trim();
                    if name.is_empty() {
                        return Err(McpError::Protocol("empty ${} placeholder".into()));
                    }
                    let expanded = lookup(name).ok_or_else(|| {
                        McpError::Protocol(format!(
                            "environment variable '{name}' referenced by MCP server env is not set"
                        ))
                    })?;
                    out.push_str(&expanded);
                    i += 2 + close + 1;
                    continue;
                }
                // Unterminated brace: treat literally (or error). Be strict.
                return Err(McpError::Protocol("unterminated ${ in MCP env value".into()));
            }
            // $NAME (up to non-name char). A variable name must start with a
            // letter or underscore, so `$5`, `$2x`, and a lone `$` are kept
            // literally rather than misread as (illegal) variable names.
            let name_end = i + 1;
            let name_bytes = &bytes[name_end..];
            let valid_start = name_bytes
                .first()
                .map(|b| b.is_ascii_alphabetic() || *b == b'_')
                .unwrap_or(false);
            if !valid_start {
                out.push('$');
                i += 1;
                continue;
            }
            let mut j = 0;
            while j < name_bytes.len()
                && (name_bytes[j].is_ascii_alphanumeric() || name_bytes[j] == b'_')
            {
                j += 1;
            }
            let name = &value[name_end..name_end + j];
            let expanded = lookup(name).ok_or_else(|| {
                McpError::Protocol(format!(
                    "environment variable '{name}' referenced by MCP server env is not set"
                ))
            })?;
            out.push_str(&expanded);
            i += j + 1;
        } else {
            // Copy one UTF-8 char.
            let ch_len = utf8_len(bytes[i]);
            out.push_str(&value[i..i + ch_len]);
            i += ch_len;
        }
    }
    Ok(out)
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else if first >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Expands every value in an env map using the real process environment.
/// Returns an error naming the first unset variable encountered.
pub fn expand_env_map(
    env: &std::collections::HashMap<String, String>,
) -> Result<std::collections::HashMap<String, String>, McpError> {
    let mut out = std::collections::HashMap::with_capacity(env.len());
    for (k, v) in env {
        let expanded = expand_env_value(v, &|name| std::env::var(name).ok())?;
        out.insert(k.clone(), expanded);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(names: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| names.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn expands_braced_and_plain() {
        let e = env(&[("HOME", "/root"), ("TOK", "abc")]);
        assert_eq!(expand_env_value("${HOME}/x", &e).unwrap(), "/root/x");
        assert_eq!(expand_env_value("$TOK", &e).unwrap(), "abc");
        assert_eq!(expand_env_value("a${TOK}b", &e).unwrap(), "aabcb");
    }

    #[test]
    fn missing_variable_errors_naming_it() {
        let e = env(&[]);
        let err = expand_env_value("${NOPE}", &e).unwrap_err();
        assert!(err.to_string().contains("NOPE"), "must name var: {err}");
        // no leak of any value
        assert!(!err.to_string().contains("secret"));
    }

    #[test]
    fn no_placeholder_is_untouched_and_lone_dollar_kept() {
        let e = env(&[]);
        assert_eq!(expand_env_value("plain value", &e).unwrap(), "plain value");
        assert_eq!(expand_env_value("cost is $5", &e).unwrap(), "cost is $5");
    }

    #[test]
    fn utf8_passthrough_is_char_safe() {
        let e = env(&[("HOME", "/root")]);
        // CJK between expansions must not be mangled.
        assert_eq!(expand_env_value("${HOME}/你/好", &e).unwrap(), "/root/你/好");
    }
}
