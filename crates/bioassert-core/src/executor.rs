use crate::assertion_request::AssertionRequest;
use crate::errors::BioAssertError;
use crate::values::Value;

pub struct AssertionExecutionResult {
    pub success: bool,
    pub actual: Value,
}

pub trait AssertionExecutor {
    fn try_parse(metric: &str) -> Option<Self>
    where
        Self: Sized;
    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError>;
}
