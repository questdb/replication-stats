use std::fmt::{Display, Write};

/// Wrapper around a byte slice that implements Display to print the bytes as a
/// Rust byte string literal. This can also be pasted into a Python REPL for
/// debugging (same format supported).
///
/// Example:
///
/// ```
/// use crate::literal_bytes::LiteralBytes;
/// let hw = &[72u8, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33];
/// let expected = "b\"Hello, World!\"";
/// let actual = format!("{}", LiteralBytes(hw));
/// assert_eq!(expected, actual);
/// ```
pub struct LiteralBytes<'a>(pub &'a [u8]);

impl Display for LiteralBytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("b\"")?;
        for byte in self.0 {
            let escaped = std::ascii::escape_default(*byte);
            for c in escaped {
                f.write_char(c as char)?; // safe because ASCII
            }
        }
        f.write_str("\"")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_bytes() {
        let hw = b"Hello, World!";
        let expected = "b\"Hello, World!\"";
        let actual = format!("{}", LiteralBytes(hw));
        assert_eq!(expected, actual);

        let test_sequence = [
            // byte, escaped
            (0, "\\x00"),
            (1, "\\x01"),
            (2, "\\x02"),
            (3, "\\x03"),
            (10, "\\n"),
            (32, " "),
            (34, "\\\""),
            (39, "\\'"),
            (92, "\\\\"),
            (97, "a"),
            (127, "\\x7f"),
            (128, "\\x80"),
            (129, "\\x81"),
            (254, "\\xfe"),
            (255, "\\xff"),
        ];
        let bytes = test_sequence.iter().map(|(b, _)| *b).collect::<Vec<_>>();

        let mut expected = "b\"".to_string();
        expected.push_str(
            &test_sequence
                .iter()
                .map(|(_, e)| *e)
                .collect::<Vec<_>>()
                .join(""),
        );
        expected.push('"');

        let actual = LiteralBytes(&bytes).to_string();
        assert_eq!(expected, actual);
    }
}
