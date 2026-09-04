//! Rendering-boundary protection for untrusted model/session content.
//! Canonical SQLite content is never passed through this function before storage.

const MAX_DISCARD: usize = 64 * 1024;

pub fn sanitize_untrusted_output(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x1b => {
                i += 1;
                i = discard_escape(bytes, i);
            }
            b'\n' | b'\t' => {
                out.push(bytes[i] as char);
                i += 1;
            }
            // CR, backspace, bell, DEL, and the remaining C0 controls can
            // alter terminal state or presentation, so they are dropped.
            0x00..=0x1f | 0x7f => i += 1,
            _ => {
                let start = i;
                while i < bytes.len() && bytes[i] != 0x1b && !(bytes[i] < 0x20 || bytes[i] == 0x7f)
                {
                    i += 1;
                }
                out.push_str(&String::from_utf8_lossy(&bytes[start..i]));
            }
        }
    }
    out
}

fn discard_escape(bytes: &[u8], mut i: usize) -> usize {
    if i >= bytes.len() {
        return i;
    }
    match bytes[i] {
        // CSI: ESC [ parameters/intermediates final-byte.
        b'[' => {
            i += 1;
            let end = (i + MAX_DISCARD).min(bytes.len());
            while i < end {
                let byte = bytes[i];
                i += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
        }
        // OSC: ESC ] payload terminated by BEL or ST.
        b']' => {
            i += 1;
            let end = (i + MAX_DISCARD).min(bytes.len());
            let mut terminated = false;
            while i < end {
                match bytes[i] {
                    0x07 => {
                        i += 1;
                        terminated = true;
                        break;
                    }
                    0x1b if bytes.get(i + 1) == Some(&b'\\') => {
                        i += 2;
                        terminated = true;
                        break;
                    }
                    _ => i += 1,
                }
            }
            if !terminated && i == end {
                return bytes.len();
            }
        }
        // DCS, SOS, PM, APC: ESC P/X/^/_ payload terminated by ST.
        b'P' | b'X' | b'^' | b'_' => {
            i += 1;
            let end = (i + MAX_DISCARD).min(bytes.len());
            let mut terminated = false;
            while i < end {
                if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    terminated = true;
                    break;
                }
                i += 1;
            }
            if !terminated && i == end {
                return bytes.len();
            }
        }
        // Generic two-byte escape sequence.
        _ => i += 1,
    }
    i
}

#[cfg(test)]
mod tests {
    use super::sanitize_untrusted_output as s;

    #[test]
    fn strips_actual_csi() {
        assert_eq!(s("a\x1b[31mred\x1b[0mb"), "aredb");
    }
    #[test]
    fn strips_actual_osc() {
        assert_eq!(s("a\x1b]8;;https://evil\x07link\x1b]8;;\x07b"), "alinkb");
    }
    #[test]
    fn strips_actual_dcs() {
        assert_eq!(s("a\x1bPsecret payload\x1b\\b"), "ab");
    }
    #[test]
    fn preserves_literal_backslash_x1b() {
        assert_eq!(s(r"\x1b[31m"), r"\x1b[31m");
    }
    #[test]
    fn preserves_newline_and_tab() {
        assert_eq!(s("a\n\tb"), "a\n\tb");
    }
    #[test]
    fn strips_cr_backspace_bell() {
        assert_eq!(s("a\rb\x08c\x07d"), "abcd");
    }
    #[test]
    fn handles_truncated_sequence() {
        assert_eq!(s("safe\x1b[31"), "safe");
    }
    #[test]
    fn preserves_unicode() {
        assert_eq!(s("こんにちは 🌍"), "こんにちは 🌍");
    }
    #[test]
    fn idempotent() {
        let x = "a\x1b[2J\nb";
        assert_eq!(s(&s(x)), s(x));
    }
    #[test]
    fn bounded_osc_payload() {
        let input = format!("a\x1b]{}b", "x".repeat(65 * 1024));
        assert_eq!(s(&input), "a");
    }
}
