use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

pub struct FileExistsExecutor;

impl AssertionExecutor for FileExistsExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.exists").then_some(Self)
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let actual = super::functions::exists(&request.file);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
