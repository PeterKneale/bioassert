use std::fmt::{Display, Formatter};
use crate::assertions::comparator_errors::ComparatorError;
use crate::assertions::comparator_errors::ComparatorError::UnknownComparator;

#[derive(Clone, Copy, PartialEq)]
pub enum Comparator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Starts,
    Ends,
    Contains,
    Matches,
}
pub fn parse_comparator(s: &str) -> Result<Comparator, ComparatorError> {
    match s {
        "eq" => Ok(Comparator::Eq),
        "ne" => Ok(Comparator::Ne),
        "lt" => Ok(Comparator::Lt),
        "lte" => Ok(Comparator::Le),
        "gt" => Ok(Comparator::Gt),
        "gte" => Ok(Comparator::Ge),
        "starts" => Ok(Comparator::Starts),
        "ends" => Ok(Comparator::Ends),
        "contains" => Ok(Comparator::Contains),
        "matches" => Ok(Comparator::Matches),
        _ => {
            let message = format!(
                "unknown comparator: {} (expected: eq, ne, lt, lte, gt, gte, starts, ends, contains, matches)",
                s
            );
            Err(UnknownComparator(message))
        }
    }
}
pub fn check_supports_boolean_comparison(comparator: Comparator) -> Result<(), ComparatorError> {
    check_supported(comparator, &[Comparator::Eq, Comparator::Ne])
}
pub fn check_supported(comparator: Comparator, supported: &[Comparator],) -> Result<(), ComparatorError> {
    if !supported.contains(&comparator) {
        let expected = supported
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ComparatorError::UnsupportedComparator(format!(
            "unsupported comparator: {} (expected: {})",
            comparator, expected
        )));
    }
    Ok(())
}
impl Comparator {
    pub fn compare<T>(self, actual: T, expected: T) -> bool
    where
        T: PartialOrd + PartialEq,
    {
        match self {
            Self::Eq => actual == expected,
            Self::Ne => actual != expected,
            Self::Lt => actual < expected,
            Self::Le => actual <= expected,
            Self::Gt => actual > expected,
            Self::Ge => actual >= expected,
            _ => false,
        }
    }

    pub fn compare_string(self, actual: &str, expected: &str) -> Result<bool, Box<dyn std::error::Error>> {
        match self {
            Self::Eq => Ok(actual == expected),
            Self::Ne => Ok(actual != expected),
            Self::Starts => Ok(actual.starts_with(expected)),
            Self::Ends => Ok(actual.ends_with(expected)),
            Self::Contains => Ok(actual.contains(expected)),
            Self::Matches => {
                let re = regex::Regex::new(expected)?;
                Ok(re.is_match(actual))
            }
            _ => Err(Box::new(ComparatorError::UnsupportedComparator(format!(
                "unsupported comparator for string comparison: {}",
                self
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_comparator

    #[test]
    fn parse_comparator_parses_all_numeric() {
        assert!(matches!(parse_comparator("eq"), Ok(Comparator::Eq)));
        assert!(matches!(parse_comparator("ne"), Ok(Comparator::Ne)));
        assert!(matches!(parse_comparator("lt"), Ok(Comparator::Lt)));
        assert!(matches!(parse_comparator("lte"), Ok(Comparator::Le)));
        assert!(matches!(parse_comparator("gt"), Ok(Comparator::Gt)));
        assert!(matches!(parse_comparator("gte"), Ok(Comparator::Ge)));
    }

    #[test]
    fn parse_comparator_parses_all_string() {
        assert!(matches!(parse_comparator("starts"), Ok(Comparator::Starts)));
        assert!(matches!(parse_comparator("ends"), Ok(Comparator::Ends)));
        assert!(matches!(parse_comparator("contains"), Ok(Comparator::Contains)));
        assert!(matches!(parse_comparator("matches"), Ok(Comparator::Matches)));
    }

    #[test]
    fn parse_comparator_rejects_unknown() {
        assert!(matches!(parse_comparator("like"), Err(ComparatorError::UnknownComparator(_))));
    }

    // compare

    #[test]
    fn compare_eq() {
        assert!(Comparator::Eq.compare(5u64, 5u64));
        assert!(!Comparator::Eq.compare(5u64, 6u64));
    }

    #[test]
    fn compare_ne() {
        assert!(Comparator::Ne.compare(5u64, 6u64));
        assert!(!Comparator::Ne.compare(5u64, 5u64));
    }

    #[test]
    fn compare_lt() {
        assert!(Comparator::Lt.compare(4u64, 5u64));
        assert!(!Comparator::Lt.compare(5u64, 5u64));
    }

    #[test]
    fn compare_le() {
        assert!(Comparator::Le.compare(5u64, 5u64));
        assert!(Comparator::Le.compare(4u64, 5u64));
        assert!(!Comparator::Le.compare(6u64, 5u64));
    }

    #[test]
    fn compare_gt() {
        assert!(Comparator::Gt.compare(6u64, 5u64));
        assert!(!Comparator::Gt.compare(5u64, 5u64));
    }

    #[test]
    fn compare_ge() {
        assert!(Comparator::Ge.compare(5u64, 5u64));
        assert!(Comparator::Ge.compare(6u64, 5u64));
        assert!(!Comparator::Ge.compare(4u64, 5u64));
    }

    // compare_string

    #[test]
    fn compare_string_eq() {
        assert!(Comparator::Eq.compare_string("hello", "hello").unwrap());
        assert!(!Comparator::Eq.compare_string("hello", "world").unwrap());
    }

    #[test]
    fn compare_string_ne() {
        assert!(Comparator::Ne.compare_string("hello", "world").unwrap());
        assert!(!Comparator::Ne.compare_string("hello", "hello").unwrap());
    }

    #[test]
    fn compare_string_starts() {
        assert!(Comparator::Starts.compare_string("Alice", "Al").unwrap());
        assert!(!Comparator::Starts.compare_string("Alice", "li").unwrap());
    }

    #[test]
    fn compare_string_ends() {
        assert!(Comparator::Ends.compare_string("Alice", "ce").unwrap());
        assert!(!Comparator::Ends.compare_string("Alice", "Al").unwrap());
    }

    #[test]
    fn compare_string_contains() {
        assert!(Comparator::Contains.compare_string("Alice", "lic").unwrap());
        assert!(!Comparator::Contains.compare_string("Alice", "xyz").unwrap());
    }

    #[test]
    fn compare_string_matches_valid_regex() {
        assert!(Comparator::Matches.compare_string("Alice", "^A.*e$").unwrap());
        assert!(!Comparator::Matches.compare_string("Bob", "^A").unwrap());
    }

    #[test]
    fn compare_string_matches_invalid_regex_returns_err() {
        assert!(Comparator::Matches.compare_string("Alice", "[invalid").is_err());
    }

    #[test]
    fn compare_string_numeric_comparator_returns_err() {
        assert!(Comparator::Lt.compare_string("a", "b").is_err());
    }

    // Display

    #[test]
    fn display_numeric_comparators() {
        assert_eq!(Comparator::Eq.to_string(), "==");
        assert_eq!(Comparator::Ne.to_string(), "!=");
        assert_eq!(Comparator::Lt.to_string(), "<");
        assert_eq!(Comparator::Le.to_string(), "<=");
        assert_eq!(Comparator::Gt.to_string(), ">");
        assert_eq!(Comparator::Ge.to_string(), ">=");
    }

    #[test]
    fn display_string_comparators() {
        assert_eq!(Comparator::Starts.to_string(), "starts_with");
        assert_eq!(Comparator::Ends.to_string(), "ends_with");
        assert_eq!(Comparator::Contains.to_string(), "contains");
        assert_eq!(Comparator::Matches.to_string(), "matches");
    }
}

impl Display for Comparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Starts => "starts_with",
            Self::Ends => "ends_with",
            Self::Contains => "contains",
            Self::Matches => "matches",
        };

        write!(f, "{s}")
    }
}
