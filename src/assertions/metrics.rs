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
        s if s.starts_with("csv.") || s.starts_with("tsv.") || s.starts_with("psv.") => {
            parse_delimited_metric(s).ok_or_else(|| {
                MetricError::UnknownMetric(format!(
                    "unknown delimited metric: {} (expected: <csv|tsv|psv>.columns.count, .lines.count, or .line.N.column.M)",
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
        s if *s == "psv" => '|',
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

#[cfg(test)]
mod tests {
    use super::*;

    // parse_metric — file metrics

    #[test]
    fn parse_metric_parses_file_metrics() {
        assert!(matches!(parse_metric("file.exists"), Ok(Metric::FileExists)));
        assert!(matches!(parse_metric("file.size"),   Ok(Metric::FileSize)));
        assert!(matches!(parse_metric("file.empty"),  Ok(Metric::FileEmpty)));
        assert!(matches!(parse_metric("file.lines"),  Ok(Metric::FileLines)));
    }

    #[test]
    fn parse_metric_rejects_unknown() {
        assert!(matches!(parse_metric("file.foo"), Err(MetricError::UnknownMetric(_))));
        assert!(matches!(parse_metric("unknown"),  Err(MetricError::UnknownMetric(_))));
    }

    // parse_metric — delimited column and line counts

    #[test]
    fn parse_metric_csv_column_count() {
        assert!(matches!(parse_metric("csv.columns.count"), Ok(Metric::DelimitedColumnCount(','))));
    }

    #[test]
    fn parse_metric_csv_line_count() {
        assert!(matches!(parse_metric("csv.lines.count"), Ok(Metric::DelimitedLineCount(','))));
    }

    #[test]
    fn parse_metric_tsv_column_count() {
        assert!(matches!(parse_metric("tsv.columns.count"), Ok(Metric::DelimitedColumnCount('\t'))));
    }

    #[test]
    fn parse_metric_tsv_line_count() {
        assert!(matches!(parse_metric("tsv.lines.count"), Ok(Metric::DelimitedLineCount('\t'))));
    }

    #[test]
    fn parse_metric_psv_column_count() {
        assert!(matches!(parse_metric("psv.columns.count"), Ok(Metric::DelimitedColumnCount('|'))));
    }

    #[test]
    fn parse_metric_psv_line_count() {
        assert!(matches!(parse_metric("psv.lines.count"), Ok(Metric::DelimitedLineCount('|'))));
    }

    // parse_metric — delimited cells

    #[test]
    fn parse_metric_csv_cell() {
        let m = parse_metric("csv.line.2.column.3").unwrap();
        assert!(matches!(m, Metric::DelimitedCell(',', 2, 3)));
    }

    #[test]
    fn parse_metric_tsv_cell() {
        let m = parse_metric("tsv.line.1.column.1").unwrap();
        assert!(matches!(m, Metric::DelimitedCell('\t', 1, 1)));
    }

    #[test]
    fn parse_metric_psv_cell() {
        let m = parse_metric("psv.line.10.column.5").unwrap();
        assert!(matches!(m, Metric::DelimitedCell('|', 10, 5)));
    }

    #[test]
    fn parse_metric_rejects_zero_line_index() {
        assert!(parse_metric("csv.line.0.column.1").is_err());
    }

    #[test]
    fn parse_metric_rejects_zero_column_index() {
        assert!(parse_metric("csv.line.1.column.0").is_err());
    }

    #[test]
    fn parse_metric_rejects_malformed_delimited() {
        assert!(parse_metric("csv.line.1").is_err());
        assert!(parse_metric("csv.unknown.thing").is_err());
    }

    // Display

    #[test]
    fn display_file_metrics() {
        assert_eq!(Metric::FileExists.to_string(), "file.exists");
        assert_eq!(Metric::FileSize.to_string(),   "file.size");
        assert_eq!(Metric::FileEmpty.to_string(),  "file.empty");
        assert_eq!(Metric::FileLines.to_string(),  "file.lines");
    }

    #[test]
    fn display_delimited_counts() {
        assert_eq!(Metric::DelimitedColumnCount(',').to_string(),  "csv.columns.count");
        assert_eq!(Metric::DelimitedLineCount(',').to_string(),    "csv.lines.count");
        assert_eq!(Metric::DelimitedColumnCount('\t').to_string(), "tsv.columns.count");
        assert_eq!(Metric::DelimitedLineCount('\t').to_string(),   "tsv.lines.count");
        assert_eq!(Metric::DelimitedColumnCount('|').to_string(),  "psv.columns.count");
        assert_eq!(Metric::DelimitedLineCount('|').to_string(),    "psv.lines.count");
    }

    #[test]
    fn display_delimited_cell() {
        assert_eq!(Metric::DelimitedCell(',',  2, 3).to_string(), "csv.line.2.column.3");
        assert_eq!(Metric::DelimitedCell('\t', 1, 1).to_string(), "tsv.line.1.column.1");
        assert_eq!(Metric::DelimitedCell('|',  10, 5).to_string(), "psv.line.10.column.5");
    }
}

fn delimiter_prefix(d: char) -> &'static str {
    match d {
        ',' => "csv",
        '\t' => "tsv",
        '|' => "psv",
        _ => "delimited",
    }
}
