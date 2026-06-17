use crate::assertions::{parse_comparator, parse_integer, BioAssertError, FileError};
use crate::metrics::MetricExecutor;
use crate::parser::Assertion;
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

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = super::functions::column_count(&file, self.delimiter)
            .map_err(|e| FileError::new(&file, e))?;
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
