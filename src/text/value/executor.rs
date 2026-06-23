use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

/// `text.value` — the literal resource compared as a string. The locator (already
/// quote-stripped by the engine) is the value; the expected side is unwrapped here, as
/// other string executors do, so `'NC_000001.11' text.value eq 'NC_000001.11'` holds.
pub struct TextValueExecutor;

impl AssertionExecutor for TextValueExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "text.value").then_some(Self)
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected);
        let success = super::functions::value_matches(&request.locator, request.comparator, expected)?;
        Ok(AssertionExecutionResult { success, actual: Value::StringValue(request.locator.clone()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_matches_text_value() {
        assert!(TextValueExecutor::try_parse("text.value").is_some());
    }

    #[test]
    fn try_parse_rejects_other_metrics() {
        assert!(TextValueExecutor::try_parse("text.length").is_none());
        assert!(TextValueExecutor::try_parse("file.size").is_none());
    }
}
