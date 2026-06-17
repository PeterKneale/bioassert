use crate::assertions::FileError;
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::metrics::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::parse_boolean;
use std::path::PathBuf;

pub struct FileEmptyExecutor;

impl MetricExecutor for FileEmptyExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.empty").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = assertion.comparator.parse::<Comparator>()?;
        let expected = parse_boolean(assertion.expected.as_str())?;
        let actual = super::functions::empty(&file).map_err(|e| FileError::new(&file, e))?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
    }
}
