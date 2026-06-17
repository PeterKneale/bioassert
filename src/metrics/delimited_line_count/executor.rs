use crate::assertions::FileError;
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::metrics::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::parse_integer;
use std::path::PathBuf;

pub struct DelimitedLineCountExecutor;

impl MetricExecutor for DelimitedLineCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        super::super::delimited_utils::delimiter_for_prefix(prefix)?;
        (rest == "lines.count").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = assertion.comparator.parse::<Comparator>()?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = super::functions::line_count(&file).map_err(|e| FileError::new(&file, e))?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
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
