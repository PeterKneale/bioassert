use crate::assertions::{AssertionExecutionResult, AssertionExecutor};
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;

pub struct DelimitedColumnCountExecutor {
    pub delimiter: char,
}

impl AssertionExecutor for DelimitedColumnCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        let delimiter = super::super::functions::delimiter_for_prefix(prefix)?;
        (rest == "columns.count").then_some(Self { delimiter })
    }

    fn execute(self, assertion: &Assertion) -> Result<AssertionExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator: Comparator = assertion.comparator.parse()?;
        let expected = Value::from_integer(&assertion.expected)?;
        let actual = super::functions::column_count(&file, self.delimiter)
            ?;
        let success = comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assertions::AssertionExecutor;

    #[test]
    fn try_parse_csv_columns_count() {
        assert!(matches!(
            DelimitedColumnCountExecutor::try_parse("csv.columns.count"),
            Some(DelimitedColumnCountExecutor { delimiter: ',' })
        ));
    }

    #[test]
    fn try_parse_tsv_columns_count() {
        assert!(matches!(
            DelimitedColumnCountExecutor::try_parse("tsv.columns.count"),
            Some(DelimitedColumnCountExecutor { delimiter: '\t' })
        ));
    }

    #[test]
    fn try_parse_rejects_unknown_prefix() {
        assert!(DelimitedColumnCountExecutor::try_parse("dsv.columns.count").is_none());
    }

    #[test]
    fn try_parse_rejects_wrong_suffix() {
        assert!(DelimitedColumnCountExecutor::try_parse("csv.lines.count").is_none());
    }
}
