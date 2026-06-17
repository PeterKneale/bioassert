use crate::assertions::{ComparatorError, MetricError, ValueParseError};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum BioAssertError {
    Comparator(ComparatorError),
    Metric(MetricError),
    ValueParse(ValueParseError),
}

impl Error for BioAssertError {}

impl Display for BioAssertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Comparator(err) => write!(f, "{err}"),
            Self::Metric(err) => write!(f, "{err}"),
            Self::ValueParse(err) => write!(f, "{err}")
        }
    }
}


impl From<ValueParseError> for BioAssertError {
    fn from(err: ValueParseError) -> Self {
        Self::ValueParse(err)
    }
}

impl From<ComparatorError> for BioAssertError {
    fn from(err: ComparatorError) -> Self {
        Self::Comparator(err)
    }
}

impl From<MetricError> for BioAssertError {
    fn from(err: MetricError) -> Self { Self::Metric(err) }
}
