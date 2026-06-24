use crate::bam::functions;
use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};

/// Counts header records of a given type: `bam.header.rg.count`, `bam.header.sq.count`,
/// `bam.header.pg.count`.
pub struct BamCountExecutor {
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    ReadGroup,
    Reference,
    Program,
}

impl AssertionExecutor for BamCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let kind = match parts.as_slice() {
            ["bam", "header", "rg", "count"] => Kind::ReadGroup,
            ["bam", "header", "sq", "count"] => Kind::Reference,
            ["bam", "header", "pg", "count"] => Kind::Program,
            _ => return None,
        };
        Some(Self { kind })
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_integer(&request.expected)?;
        let header = functions::read_header(request.path())?;
        let count = match self.kind {
            Kind::ReadGroup => functions::read_group_count(&header),
            Kind::Reference => functions::reference_count(&header),
            Kind::Program => functions::program_count(&header),
        };
        let actual = Value::IntegerValue(count);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_accepts_counts() {
        assert!(BamCountExecutor::try_parse("bam.header.rg.count").is_some());
        assert!(BamCountExecutor::try_parse("bam.header.sq.count").is_some());
        assert!(BamCountExecutor::try_parse("bam.header.pg.count").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown() {
        assert!(BamCountExecutor::try_parse("bam.header.hd.count").is_none());
        assert!(BamCountExecutor::try_parse("bam.header.rg.total").is_none());
        assert!(BamCountExecutor::try_parse("bam.header.rg.0.sm").is_none());
        // the old un-nested form no longer parses
        assert!(BamCountExecutor::try_parse("bam.rg.count").is_none());
    }
}
