use super::errors::ComparatorError;
use super::errors::ComparatorError::UnknownComparator;
use crate::errors::BioAssertError;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

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

    pub fn compare_string(self, actual: &str, expected: &str) -> Result<bool, BioAssertError> {
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
            _ => Err(ComparatorError::UnsupportedComparator(format!(
                "unsupported comparator for string comparison: {}",
                self
            ))
            .into()),
        }
    }
}

impl FromStr for Comparator {
    type Err = ComparatorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eq" => Ok(Self::Eq),
            "ne" => Ok(Self::Ne),
            "lt" => Ok(Self::Lt),
            "lte" => Ok(Self::Le),
            "gt" => Ok(Self::Gt),
            "gte" => Ok(Self::Ge),
            "starts" => Ok(Self::Starts),
            "ends" => Ok(Self::Ends),
            "contains" => Ok(Self::Contains),
            "matches" => Ok(Self::Matches),
            _ => Err(UnknownComparator(format!(
                "unknown comparator: {} (expected: eq, ne, lt, lte, gt, gte, starts, ends, contains, matches)",
                s
            ))),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_parses_all_numeric() {
        assert!(matches!("eq".parse::<Comparator>(), Ok(Comparator::Eq)));
        assert!(matches!("ne".parse::<Comparator>(), Ok(Comparator::Ne)));
        assert!(matches!("lt".parse::<Comparator>(), Ok(Comparator::Lt)));
        assert!(matches!("lte".parse::<Comparator>(), Ok(Comparator::Le)));
        assert!(matches!("gt".parse::<Comparator>(), Ok(Comparator::Gt)));
        assert!(matches!("gte".parse::<Comparator>(), Ok(Comparator::Ge)));
    }

    #[test]
    fn from_str_parses_all_string() {
        assert!(matches!("starts".parse::<Comparator>(), Ok(Comparator::Starts)));
        assert!(matches!("ends".parse::<Comparator>(), Ok(Comparator::Ends)));
        assert!(matches!("contains".parse::<Comparator>(), Ok(Comparator::Contains)));
        assert!(matches!("matches".parse::<Comparator>(), Ok(Comparator::Matches)));
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!(matches!("like".parse::<Comparator>(), Err(ComparatorError::UnknownComparator(_))));
    }

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
