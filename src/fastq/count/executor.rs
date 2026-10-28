use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};
use crate::fastq::functions;

/// Counts records (`fastq.read.count`) or sums bases across every record (`fastq.length`).
/// An invalid file is an error carrying the reason.
pub struct FastqCountExecutor {
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    Count,
    Length,
}

impl AssertionExecutor for FastqCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let kind = match parts.as_slice() {
            ["fastq", "read", "count"] => Kind::Count,
            ["fastq", "length"] => Kind::Length,
            _ => return None,
        };
        Some(Self { kind })
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_integer(&request.expected)?;
        let scan = functions::scan_valid(request.path())?;
        let value = match self.kind {
            Kind::Count => scan.reads,
            Kind::Length => scan.bases,
        };
        let actual = Value::IntegerValue(value);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult {
            success,
            actual,
            detail: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_accepts_count_and_length() {
        assert!(FastqCountExecutor::try_parse("fastq.read.count").is_some());
        assert!(FastqCountExecutor::try_parse("fastq.length").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown() {
        assert!(FastqCountExecutor::try_parse("fastq.count").is_none());
        assert!(FastqCountExecutor::try_parse("fastq.read.length").is_none());
        assert!(FastqCountExecutor::try_parse("fasta.length").is_none());
    }
}
