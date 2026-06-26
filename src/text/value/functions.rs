use crate::core::{BioAssertError, Comparator};

/// A text resource's value is the locator string itself. This compares it against the
/// expected value with the string comparators (`eq`, `ne`, `starts`, `ends`, `contains`,
/// `matches`). Errors only if the comparator is a numeric one (not valid for strings) or
/// the `matches` regex is invalid; it never errors on the resource, which is always present.
pub fn value_matches(
    value: &str,
    comparator: Comparator,
    expected: &str,
) -> Result<bool, BioAssertError> {
    comparator.compare_string(value, expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Operator;

    #[test]
    fn eq_matches_identical() {
        assert!(value_matches("abc", Operator::Eq.into(), "abc").unwrap());
        assert!(!value_matches("abc", Operator::Eq.into(), "abd").unwrap());
    }

    #[test]
    fn ne_matches_different() {
        assert!(value_matches("abc", Operator::Ne.into(), "xyz").unwrap());
        assert!(!value_matches("abc", Operator::Ne.into(), "abc").unwrap());
    }

    #[test]
    fn starts_ends_contains() {
        assert!(value_matches("abc", Operator::Starts.into(), "ab").unwrap());
        assert!(value_matches("abc", Operator::Ends.into(), "bc").unwrap());
        assert!(value_matches("abc", Operator::Contains.into(), "b").unwrap());
        assert!(!value_matches("abc", Operator::Starts.into(), "bc").unwrap());
    }

    #[test]
    fn matches_regex_with_dots_and_anchors() {
        assert!(value_matches("NC_000001.11", Operator::Matches.into(), r"^NC_\d+\.\d+$").unwrap());
        assert!(!value_matches("chr1", Operator::Matches.into(), r"^NC_").unwrap());
    }

    #[test]
    fn numeric_comparator_errors() {
        assert!(value_matches("abc", Operator::Gt.into(), "5").is_err());
    }
}
