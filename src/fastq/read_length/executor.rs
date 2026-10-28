use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};
use crate::fastq::functions;

/// The shortest (`fastq.read.length.min`) or longest (`fastq.read.length.max`) read. Both are
/// `0` on an empty file. An invalid file is an error carrying the reason.
pub struct FastqReadLengthExecutor {
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    Min,
    Max,
}

impl AssertionExecutor for FastqReadLengthExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let kind = match parts.as_slice() {
            ["fastq", "read", "length", "min"] => Kind::Min,
            ["fastq", "read", "length", "max"] => Kind::Max,
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
            Kind::Min => scan.min_length,
            Kind::Max => scan.max_length,
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
    fn try_parse_accepts_min_and_max() {
        assert!(FastqReadLengthExecutor::try_parse("fastq.read.length.min").is_some());
        assert!(FastqReadLengthExecutor::try_parse("fastq.read.length.max").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown() {
        assert!(FastqReadLengthExecutor::try_parse("fastq.read.length").is_none());
        assert!(FastqReadLengthExecutor::try_parse("fastq.read.length.mean").is_none());
        assert!(FastqReadLengthExecutor::try_parse("fastq.read.0.length").is_none());
    }
}
