use crate::assertions::FileError;
use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::metrics::{ExecutionResult, MetricExecutor};
use crate::parser::Assertion;
use crate::values::parse_bytes;
use std::path::PathBuf;

pub struct FileSizeExecutor;

impl MetricExecutor for FileSizeExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.size").then_some(Self)
    }

    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = assertion.comparator.parse::<Comparator>()?;
        let expected = parse_bytes(assertion.expected.as_str())?;
        let actual = super::functions::size(&file).map_err(|e| FileError::new(&file, e))?;
        let success = comparator.compare(&actual, &expected);
        Ok(ExecutionResult { success, actual })
    }
}
