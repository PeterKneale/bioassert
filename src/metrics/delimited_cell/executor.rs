use crate::assertions::{parse_comparator, BioAssertError, FileError};
use crate::metrics::MetricExecutor;
use crate::parser::Assertion;
use std::path::PathBuf;

pub struct DelimitedCellExecutor {
    pub delimiter: char,
    pub line: usize,
    pub col: usize,
}

fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 {
        let b = s.as_bytes();
        if (b[0] == b'\'' && b[s.len() - 1] == b'\'') || (b[0] == b'"' && b[s.len() - 1] == b'"')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

impl MetricExecutor for DelimitedCellExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            [prefix, "line", n, "column", m] => {
                let delimiter =
                    super::super::delimited_utils::delimiter_for_prefix(prefix)?;
                let line = n.parse::<usize>().ok().filter(|&x| x > 0)?;
                let col = m.parse::<usize>().ok().filter(|&x| x > 0)?;
                Some(Self { delimiter, line, col })
            }
            _ => None,
        }
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected_str = strip_quotes(&assertion.expected).to_string();
        let actual =
            super::functions::cell(&file, self.delimiter, self.line, self.col)
                .map_err(|e| FileError::new(&file, e))?;
        let result = comparator.compare_string(&actual, &expected_str)?;
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected_str, actual
        );
        Ok((result, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricExecutor;

    #[test]
    fn strip_quotes_removes_double_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn strip_quotes_removes_single_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_unquoted_string() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_short_string() {
        assert_eq!(strip_quotes("a"), "a");
        assert_eq!(strip_quotes(""), "");
    }

    #[test]
    fn try_parse_csv_cell() {
        assert!(matches!(
            DelimitedCellExecutor::try_parse("csv.line.2.column.3"),
            Some(DelimitedCellExecutor { delimiter: ',', line: 2, col: 3 })
        ));
    }

    #[test]
    fn try_parse_tsv_cell() {
        assert!(matches!(
            DelimitedCellExecutor::try_parse("tsv.line.1.column.1"),
            Some(DelimitedCellExecutor { delimiter: '\t', line: 1, col: 1 })
        ));
    }

    #[test]
    fn try_parse_rejects_zero_line() {
        assert!(DelimitedCellExecutor::try_parse("csv.line.0.column.1").is_none());
    }

    #[test]
    fn try_parse_rejects_zero_column() {
        assert!(DelimitedCellExecutor::try_parse("csv.line.1.column.0").is_none());
    }

    #[test]
    fn try_parse_rejects_unknown_prefix() {
        assert!(DelimitedCellExecutor::try_parse("dsv.line.1.column.1").is_none());
    }
}
