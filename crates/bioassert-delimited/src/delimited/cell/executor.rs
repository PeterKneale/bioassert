use bioassert_core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

pub struct DelimitedCellExecutor {
    pub delimiter: char,
    pub line: usize,
    pub col: usize,
}

impl AssertionExecutor for DelimitedCellExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            [prefix, "line", n, "column", m] => {
                let delimiter = super::super::functions::delimiter_for_prefix(prefix)?;
                let line = n.parse::<usize>().ok().filter(|&x| x > 0)?;
                let col = m.parse::<usize>().ok().filter(|&x| x > 0)?;
                Some(Self { delimiter, line, col })
            }
            _ => None,
        }
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected_str = super::functions::strip_quotes(&request.expected).to_string();
        let actual_str = super::functions::cell(&request.file, self.delimiter, self.line, self.col)?;
        let success = request.comparator.compare_string(&actual_str, &expected_str)?;
        Ok(AssertionExecutionResult { success, actual: Value::StringValue(actual_str) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bioassert_core::AssertionExecutor;

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
