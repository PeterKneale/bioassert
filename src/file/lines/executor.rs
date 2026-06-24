use super::functions;
use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};

pub struct FileLinesExecutor;

impl AssertionExecutor for FileLinesExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.lines").then_some(Self)
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_integer(&request.expected)?;
        let actual = functions::count_lines(request.path())?;
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}
