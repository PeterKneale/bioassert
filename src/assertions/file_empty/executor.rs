use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::assertions::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::Value;
use std::path::PathBuf;

pub struct FileEmptyExecutor;

impl MetricExecutor for FileEmptyExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.empty").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator: Comparator = assertion.comparator.parse()?;
        let expected = Value::from_boolean(&assertion.expected)?;
        let actual = super::functions::empty(&file)?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
    }
}
