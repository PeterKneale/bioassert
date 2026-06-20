use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ComparatorError {
    UnknownComparator(String),
    UnsupportedComparator(String),
}

impl Error for ComparatorError {}

impl Display for ComparatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparatorError::UnknownComparator(message) => write!(f, "{message}"),
            ComparatorError::UnsupportedComparator(message) => write!(f, "{message}"),
        }
    }
}
