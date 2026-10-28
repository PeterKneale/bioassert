use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};
use crate::fastq::functions;

/// Whether the whole file parses as four-line FASTQ records (`fastq.valid`). A malformed
/// or truncated file reports `false` rather than erroring, so this is the safe guard for
/// the other `fastq.*` metrics.
pub struct FastqValidExecutor;

impl AssertionExecutor for FastqValidExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "fastq.valid").then_some(Self)
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let scan = functions::scan(request.path())?;
        let actual = Value::BooleanValue(scan.is_valid());
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult {
            success,
            actual,
            detail: scan.invalid.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_accepts_valid() {
        assert!(FastqValidExecutor::try_parse("fastq.valid").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown() {
        assert!(FastqValidExecutor::try_parse("fastq.validity").is_none());
        assert!(FastqValidExecutor::try_parse("fastq.read.valid").is_none());
        assert!(FastqValidExecutor::try_parse("fasta.valid").is_none());
    }
}
