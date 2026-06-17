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
}
pub fn parse_comparator(s: &str) -> Result<Comparator, ComparatorError> {
    match s {
        "eq" => Ok(Comparator::Eq),
        "ne" => Ok(Comparator::Ne),
        "lt" => Ok(Comparator::Lt),
        "lte" => Ok(Comparator::Le),
        "gt" => Ok(Comparator::Gt),
        "gte" => Ok(Comparator::Ge),
        _ => {
            let message = format!(
                "unknown comparator: {} (expected: eq, ne, lt, le, gt, ge)",
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
        };

        write!(f, "{s}")
    }
}
