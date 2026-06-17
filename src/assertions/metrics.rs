use std::fmt::{Display, Formatter};
use crate::assertions::metrics_error::MetricError;

pub const FILE_EXISTS_METRIC: &str = "file.exists";
pub const FILE_SIZE_METRIC: &str = "file.size";
pub const FILE_EMPTY_METRIC: &str = "file.empty";
pub const FILE_LINES_METRIC: &str = "file.lines";
pub const METRICS: [&str; 4] = [FILE_EXISTS_METRIC, FILE_SIZE_METRIC, FILE_EMPTY_METRIC, FILE_LINES_METRIC];

#[derive(Clone, Copy)]
pub enum Metric {
    FileExists,
    FileSize,
    FileEmpty,
    FileLines,
}

impl Display for Metric {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::FileExists => FILE_EXISTS_METRIC,
            Self::FileSize => FILE_SIZE_METRIC,
            Self::FileEmpty => FILE_EMPTY_METRIC,
            Self::FileLines => FILE_LINES_METRIC,
        };
        write!(f, "{s}")
    }
}

pub fn parse_metric(value: &str) -> Result<Metric, MetricError> {
    match value {
        FILE_EXISTS_METRIC => Ok(Metric::FileExists),
        FILE_SIZE_METRIC => Ok(Metric::FileSize),
        FILE_EMPTY_METRIC => Ok(Metric::FileEmpty),
        FILE_LINES_METRIC => Ok(Metric::FileLines),
        _ => {
            let expected = METRICS.join(", ");
            let message = format!("unknown metric: {} (expected: {})", value, expected);
            Err(MetricError::UnknownMetric(message))
        }
    }
}

