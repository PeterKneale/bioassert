use crate::assertions::{AssertionExecutionResult, AssertionExecutor};
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;

pub struct FileExistsExecutor;

impl AssertionExecutor for FileExistsExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.exists").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<AssertionExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator: Comparator = assertion.comparator.parse()?;
        let expected = Value::from_boolean(&assertion.expected)?;
        let actual = super::functions::exists(&file);
        let success = comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
