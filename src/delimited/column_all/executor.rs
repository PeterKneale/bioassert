use super::functions::ColumnCheck;
use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

/// Asserts that a comparison holds for every cell of a delimited column.
///
/// `<prefix>.column.<n>.all` checks every line (the header included), while
/// `<prefix>.column.<n>.data.all` skips the first line so only data rows are checked.
/// Any string comparator is supported (`matches`, `eq`, `contains`, ...); the assertion
/// passes only when every checked cell satisfies it.
pub struct DelimitedColumnAllExecutor {
    pub delimiter: char,
    pub col: usize,
    pub skip_header: bool,
}

impl AssertionExecutor for DelimitedColumnAllExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let (prefix, n, skip_header) = match parts.as_slice() {
            [prefix, "column", n, "all"] => (prefix, n, false),
            [prefix, "column", n, "data", "all"] => (prefix, n, true),
            _ => return None,
        };
        let delimiter = crate::delimited::functions::delimiter_for_prefix(prefix)?;
        let col = n.parse::<usize>().ok().filter(|&x| x > 0)?;
        Some(Self { delimiter, col, skip_header })
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected);
        // Compile the comparison once, then apply it to every cell in the column.
        let matcher = request.comparator.string_matcher(expected)?;
        let check = super::functions::check_column(request.path(), self.delimiter, self.col, self.skip_header, &matcher)?;
        let (success, actual) = match check {
            ColumnCheck::AllMatch { checked: 0 } => (true, "no rows checked".to_string()),
            ColumnCheck::AllMatch { checked } => (true, format!("{checked} rows checked")),
            ColumnCheck::Mismatch { line, value } => (false, format!("line {line} = \"{value}\"")),
        };
        Ok(AssertionExecutionResult { success, actual: Value::StringValue(actual) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AssertionExecutor;

    #[test]
    fn try_parse_tsv_column_all() {
        assert!(matches!(
            DelimitedColumnAllExecutor::try_parse("tsv.column.6.all"),
            Some(DelimitedColumnAllExecutor { delimiter: '\t', col: 6, skip_header: false })
        ));
    }

    #[test]
    fn try_parse_csv_column_data_all_skips_header() {
        assert!(matches!(
            DelimitedColumnAllExecutor::try_parse("csv.column.4.data.all"),
            Some(DelimitedColumnAllExecutor { delimiter: ',', col: 4, skip_header: true })
        ));
    }

    #[test]
    fn try_parse_psv_column_all() {
        assert!(matches!(
            DelimitedColumnAllExecutor::try_parse("psv.column.1.all"),
            Some(DelimitedColumnAllExecutor { delimiter: '|', col: 1, skip_header: false })
        ));
    }

    #[test]
    fn try_parse_rejects_zero_column() {
        assert!(DelimitedColumnAllExecutor::try_parse("tsv.column.0.all").is_none());
    }

    #[test]
    fn try_parse_rejects_unknown_prefix() {
        assert!(DelimitedColumnAllExecutor::try_parse("dsv.column.1.all").is_none());
    }

    #[test]
    fn try_parse_rejects_missing_all_suffix() {
        assert!(DelimitedColumnAllExecutor::try_parse("tsv.column.1").is_none());
    }

    #[test]
    fn try_parse_rejects_non_numeric_column() {
        assert!(DelimitedColumnAllExecutor::try_parse("tsv.column.x.all").is_none());
    }
}
