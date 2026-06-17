use crate::assertions::FileError;
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::metrics::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::parse_integer;
use std::path::PathBuf;

pub struct DelimitedColumnCountExecutor {
    pub delimiter: char,
}

impl MetricExecutor for DelimitedColumnCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        let delimiter = super::super::delimited_utils::delimiter_for_prefix(prefix)?;
        (rest == "columns.count").then_some(Self { delimiter })
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = assertion.comparator.parse::<Comparator>()?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = super::functions::column_count(&file, self.delimiter)
            .map_err(|e| FileError::new(&file, e))?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricExecutor;

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
