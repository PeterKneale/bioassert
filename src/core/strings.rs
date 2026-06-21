/// Removes a single matching pair of surrounding single or double quotes, leaving any
/// other string untouched. Shared by executors whose expected value may be quoted to carry
/// characters the bare-string grammar rejects (dots, dashes, colons).
pub fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 {
        let b = s.as_bytes();
        if (b[0] == b'\'' && b[s.len() - 1] == b'\'') || (b[0] == b'"' && b[s.len() - 1] == b'"') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_quotes_removes_double_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn strip_quotes_removes_single_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_unquoted_string() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_short_string() {
        assert_eq!(strip_quotes("a"), "a");
        assert_eq!(strip_quotes(""), "");
    }
}
