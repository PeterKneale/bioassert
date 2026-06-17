use super::functions::get_file_size;
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::assertions::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;
use std::str::FromStr;

pub struct FileSizeExecutor;

impl MetricExecutor for FileSizeExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.size").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator  = Comparator::from_str(&assertion.comparator)?;
        let expected = Value::from_bytes(&assertion.expected)?;
        let actual = get_file_size(&file)?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
    }
}
