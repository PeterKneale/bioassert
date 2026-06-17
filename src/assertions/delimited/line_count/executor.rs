use crate::assertions::{AssertionExecutionResult, AssertionExecutor};
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;

pub struct DelimitedLineCountExecutor;

impl AssertionExecutor for DelimitedLineCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        super::super::functions::delimiter_for_prefix(prefix)?;
        (rest == "lines.count").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<AssertionExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator: Comparator = assertion.comparator.parse()?;
        let expected = Value::from_integer(&assertion.expected)?;
        let actual = super::functions::line_count(&file)?;
        let success = comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assertions::AssertionExecutor;

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
