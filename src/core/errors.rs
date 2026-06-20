use crate::core::comparisons::ComparatorError;
use crate::core::file_error::FileError;
use crate::core::values::ValueParseError;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum BioAssertError {
    File(FileError),
    Metric(String),
    Comparator(ComparatorError),
    Value(ValueParseError),
    Regex(regex::Error),
}

impl Error for BioAssertError {}

impl Display for BioAssertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(err) => write!(f, "{err}"),
            Self::Metric(msg) => write!(f, "unknown metric: {msg}"),
            Self::Comparator(err) => write!(f, "{err}"),
            Self::Value(err) => write!(f, "{err}"),
            Self::Regex(err) => write!(f, "invalid regex: {err}"),
        }
    }
}

impl From<FileError> for BioAssertError {
    fn from(err: FileError) -> Self {
        Self::File(err)
    }
}

impl From<ComparatorError> for BioAssertError {
    fn from(err: ComparatorError) -> Self {
        Self::Comparator(err)
    }
}

impl From<ValueParseError> for BioAssertError {
    fn from(err: ValueParseError) -> Self {
        Self::Value(err)
    }
}

impl From<regex::Error> for BioAssertError {
    fn from(err: regex::Error) -> Self {
        Self::Regex(err)
    }
}
