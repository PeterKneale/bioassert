use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value};
use crate::fasta::functions;

/// Counts sequence records (`fasta.seq.count`) or sums bases across every record
/// (`fasta.length`).
pub struct FastaCountExecutor {
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    Count,
    Length,
}

impl AssertionExecutor for FastaCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let kind = match parts.as_slice() {
            ["fasta", "seq", "count"] => Kind::Count,
            ["fasta", "length"] => Kind::Length,
            _ => return None,
        };
        Some(Self { kind })
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_integer(&request.expected)?;
        let records = functions::read_records(request.path())?;
        let value = match self.kind {
            Kind::Count => functions::record_count(&records),
            Kind::Length => functions::total_length(&records),
        };
        let actual = Value::IntegerValue(value);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_accepts_count_and_length() {
        assert!(FastaCountExecutor::try_parse("fasta.seq.count").is_some());
        assert!(FastaCountExecutor::try_parse("fasta.length").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown() {
        assert!(FastaCountExecutor::try_parse("fasta.seq.counts").is_none());
        assert!(FastaCountExecutor::try_parse("fasta.count").is_none());
        assert!(FastaCountExecutor::try_parse("fasta.seq.0.count").is_none());
        // unrelated namespaces never match
        assert!(FastaCountExecutor::try_parse("bam.header.rg.count").is_none());
    }
}
