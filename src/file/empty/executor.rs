use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

pub struct FileEmptyExecutor;

impl AssertionExecutor for FileEmptyExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.empty").then_some(Self)
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let actual = super::functions::empty(request.path())?;
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
