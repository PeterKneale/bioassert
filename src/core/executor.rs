use crate::core::assertion_request::AssertionRequest;
use crate::core::errors::BioAssertError;
use crate::core::values::Value;

pub struct AssertionExecutionResult {
    pub success: bool,
    pub actual: Value,
    /// Why the actual value is what it is, when the value alone would not tell the reader
    /// (e.g. which record made a file invalid). Appended to the report line in brackets.
    pub detail: Option<String>,
}

pub trait AssertionExecutor {
    fn try_parse(metric: &str) -> Option<Self>
    where
        Self: Sized;
    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError>;
}
