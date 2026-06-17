use crate::assertions::{parse_comparator, parse_integer, BioAssertError, FileError};
use crate::metrics::MetricExecutor;
use crate::parser::Assertion;
use std::path::PathBuf;

pub struct DelimitedLineCountExecutor;

impl MetricExecutor for DelimitedLineCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        super::super::delimited_utils::delimiter_for_prefix(prefix)?;
        (rest == "lines.count").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = super::functions::line_count(&file).map_err(|e| FileError::new(&file, e))?;
        let result = comparator.compare(&actual, &expected);
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected, actual
        );
        Ok((result, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricExecutor;

    #[test]
    fn try_parse_csv_lines_count() {
        assert!(DelimitedLineCountExecutor::try_parse("csv.lines.count").is_some());
    }

    #[test]
    fn try_parse_tsv_lines_count() {
        assert!(DelimitedLineCountExecutor::try_parse("tsv.lines.count").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown_prefix() {
        assert!(DelimitedLineCountExecutor::try_parse("dsv.lines.count").is_none());
    }

    #[test]
    fn try_parse_rejects_wrong_suffix() {
        assert!(DelimitedLineCountExecutor::try_parse("csv.columns.count").is_none());
    }
}
