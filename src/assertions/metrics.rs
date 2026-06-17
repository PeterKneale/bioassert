use std::fmt::{Display, Formatter};
use crate::assertions::metrics_error::MetricError;

pub const FILE_EXISTS_METRIC: &str = "file.exists";
pub const FILE_SIZE_METRIC: &str = "file.size";
pub const FILE_EMPTY_METRIC: &str = "file.empty";
pub const FILE_LINES_METRIC: &str = "file.lines";

#[derive(Clone, Copy)]
pub enum Metric {
    FileExists,
    FileSize,
    FileEmpty,
    FileLines,
    DelimitedColumnCount(char),
    DelimitedLineCount(char),
    DelimitedCell(char, usize, usize),
}

impl Display for Metric {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileExists => write!(f, "{FILE_EXISTS_METRIC}"),
            Self::FileSize => write!(f, "{FILE_SIZE_METRIC}"),
            Self::FileEmpty => write!(f, "{FILE_EMPTY_METRIC}"),
            Self::FileLines => write!(f, "{FILE_LINES_METRIC}"),
            Self::DelimitedColumnCount(d) => write!(f, "{}.columns.count", delimiter_prefix(*d)),
            Self::DelimitedLineCount(d) => write!(f, "{}.lines.count", delimiter_prefix(*d)),
            Self::DelimitedCell(d, line, col) => {
                write!(f, "{}.line.{}.column.{}", delimiter_prefix(*d), line, col)
            }
        }
    }
}

pub fn parse_metric(value: &str) -> Result<Metric, MetricError> {
    match value {
        FILE_EXISTS_METRIC => Ok(Metric::FileExists),
        FILE_SIZE_METRIC => Ok(Metric::FileSize),
        FILE_EMPTY_METRIC => Ok(Metric::FileEmpty),
        FILE_LINES_METRIC => Ok(Metric::FileLines),
        s if s.starts_with("csv.") || s.starts_with("tsv.") => {
            parse_delimited_metric(s).ok_or_else(|| {
                MetricError::UnknownMetric(format!(
                    "unknown delimited metric: {} (expected: csv.columns.count, csv.lines.count, csv.line.N.column.M)",
                    s
                ))
            })
        }
        _ => {
            let known = [FILE_EXISTS_METRIC, FILE_SIZE_METRIC, FILE_EMPTY_METRIC, FILE_LINES_METRIC];
            let message = format!(
                "unknown metric: {} (expected one of: {}, or csv.*/tsv.* metrics)",
                value,
                known.join(", ")
            );
            Err(MetricError::UnknownMetric(message))
        }
    }
}

fn parse_delimited_metric(s: &str) -> Option<Metric> {
    let parts: Vec<&str> = s.split('.').collect();
    let delimiter = match parts.first()? {
        s if *s == "csv" => ',',
        s if *s == "tsv" => '\t',
        _ => return None,
    };
    match parts.as_slice() {
        [_, "columns", "count"] => Some(Metric::DelimitedColumnCount(delimiter)),
        [_, "lines", "count"] => Some(Metric::DelimitedLineCount(delimiter)),
        [_, "line", n, "column", m] => {
            let line = n.parse::<usize>().ok()?;
            let col = m.parse::<usize>().ok()?;
            if line == 0 || col == 0 {
                return None;
            }
            Some(Metric::DelimitedCell(delimiter, line, col))
        }
        _ => None,
    }
}

fn delimiter_prefix(d: char) -> &'static str {
    match d {
        ',' => "csv",
        '\t' => "tsv",
        _ => "delimited",
    }
}
