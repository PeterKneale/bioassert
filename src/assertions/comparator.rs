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
