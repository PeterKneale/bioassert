use super::functions::get_file_size;
use crate::assertions::{AssertionExecutionResult, AssertionExecutor};
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;
use std::str::FromStr;

pub struct FileSizeExecutor;

impl AssertionExecutor for FileSizeExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.size").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<AssertionExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator  = Comparator::from_str(&assertion.comparator)?;
        let expected = Value::from_bytes(&assertion.expected)?;
        let actual = get_file_size(&file)?;
        let success = comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
