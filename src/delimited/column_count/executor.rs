use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};

pub struct DelimitedColumnCountExecutor {
    pub delimiter: char,
}

impl AssertionExecutor for DelimitedColumnCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        let delimiter = crate::delimited::functions::delimiter_for_prefix(prefix)?;
        (rest == "columns.count").then_some(Self { delimiter })
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_integer(&request.expected)?;
        let actual = super::functions::column_count(request.path(), self.delimiter)?;
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AssertionExecutor;

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
