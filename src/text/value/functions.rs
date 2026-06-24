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

    #[test]
    fn eq_matches_identical() {
        assert!(value_matches("abc", Comparator::Eq, "abc").unwrap());
        assert!(!value_matches("abc", Comparator::Eq, "abd").unwrap());
    }

    #[test]
    fn ne_matches_different() {
        assert!(value_matches("abc", Comparator::Ne, "xyz").unwrap());
        assert!(!value_matches("abc", Comparator::Ne, "abc").unwrap());
    }

    #[test]
    fn starts_ends_contains() {
        assert!(value_matches("abc", Comparator::Starts, "ab").unwrap());
        assert!(value_matches("abc", Comparator::Ends, "bc").unwrap());
        assert!(value_matches("abc", Comparator::Contains, "b").unwrap());
        assert!(!value_matches("abc", Comparator::Starts, "bc").unwrap());
    }

    #[test]
    fn matches_regex_with_dots_and_anchors() {
        assert!(value_matches("NC_000001.11", Comparator::Matches, r"^NC_\d+\.\d+$").unwrap());
        assert!(!value_matches("chr1", Comparator::Matches, r"^NC_").unwrap());
    }

    #[test]
    fn numeric_comparator_errors() {
        assert!(value_matches("abc", Comparator::Gt, "5").is_err());
    }
}
