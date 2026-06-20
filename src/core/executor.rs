use crate::core::assertion_request::AssertionRequest;
use crate::core::errors::BioAssertError;
use crate::core::values::Value;

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
