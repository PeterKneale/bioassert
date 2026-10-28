use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};
use crate::fastq::functions;

/// The quality encoding inferred from the range of quality characters
/// (`fastq.quality.encoding`): `phred33`, `phred64` or `ambiguous`. Compared as a string.
/// An invalid file is an error carrying the reason.
pub struct FastqQualityEncodingExecutor;

impl AssertionExecutor for FastqQualityEncodingExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "fastq.quality.encoding").then_some(Self)
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected);
        let scan = functions::scan_valid(request.path())?;
        let actual = scan.encoding().as_str();
        let success = request.comparator.compare_string(actual, expected)?;
        Ok(AssertionExecutionResult {
            success,
            actual: Value::StringValue(actual.to_string()),
            detail: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_accepts_encoding() {
        assert!(FastqQualityEncodingExecutor::try_parse("fastq.quality.encoding").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown() {
        assert!(FastqQualityEncodingExecutor::try_parse("fastq.quality.mean").is_none());
        assert!(FastqQualityEncodingExecutor::try_parse("fastq.quality").is_none());
    }
}
