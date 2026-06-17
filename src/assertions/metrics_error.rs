use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum MetricError {
    UnknownMetric(String),
}

impl Error for MetricError {}

impl Display for MetricError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMetric(message) => write!(f, "{message}"),
        }
    }
}