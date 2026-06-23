use super::functions::get_file_size;
use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

pub struct FileSizeExecutor;

impl AssertionExecutor for FileSizeExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.size").then_some(Self)
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_bytes(&request.expected)?;
        let actual = get_file_size(request.path())?;
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
