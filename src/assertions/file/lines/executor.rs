use super::functions;
use crate::assertions::{AssertionExecutionResult, AssertionExecutor};
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;

pub struct FileLinesExecutor;

impl AssertionExecutor for FileLinesExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.lines").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<AssertionExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator: Comparator = assertion.comparator.parse()?;
        let expected = Value::from_integer(&assertion.expected)?;
        let actual = functions::count_lines(&file)?;
        let success = comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
