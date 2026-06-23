use super::functions::length;
use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};

/// `text.length` — the character length of the literal resource, compared numerically.
pub struct TextLengthExecutor;

impl AssertionExecutor for TextLengthExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "text.length").then_some(Self)
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_integer(&request.expected)?;
        let actual = Value::IntegerValue(length(&request.locator));
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_matches_text_length() {
        assert!(TextLengthExecutor::try_parse("text.length").is_some());
    }

    #[test]
    fn try_parse_rejects_other_metrics() {
        assert!(TextLengthExecutor::try_parse("text.value").is_none());
        assert!(TextLengthExecutor::try_parse("file.size").is_none());
    }
}
