use crate::assertions::FileError;
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::metrics::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::parse_integer;
use std::path::PathBuf;

pub struct FileLinesExecutor;

impl MetricExecutor for FileLinesExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.lines").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = assertion.comparator.parse::<Comparator>()?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = super::functions::count_lines(&file).map_err(|e| FileError::new(&file, e))?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
    }
}
