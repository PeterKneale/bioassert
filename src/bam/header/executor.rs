use crate::bam::functions;
use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, FileError, Value};
use std::io;

/// Reads an `@HD` header field: `bam.header.hd.vn` (version) or `bam.header.hd.so` (sort
/// order). Errors when there is no `@HD` line or the field is not set.
pub struct BamHeaderFieldExecutor {
    field: String,
}

impl AssertionExecutor for BamHeaderFieldExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            ["bam", "header", "hd", field @ ("vn" | "so")] => Some(Self { field: field.to_string() }),
            _ => None,
        }
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected).to_string();
        let header = functions::read_header(request.path())?;
        let actual = functions::hd_field(&header, &self.field).ok_or_else(|| {
            FileError::new(
                request.path(),
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("@HD field {} not found", self.field),
                ),
            )
        })?;
        let success = request.comparator.compare_string(&actual, &expected)?;
        Ok(AssertionExecutionResult { success, actual: Value::StringValue(actual) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_accepts_known_fields() {
        assert!(BamHeaderFieldExecutor::try_parse("bam.header.hd.vn").is_some());
        assert!(BamHeaderFieldExecutor::try_parse("bam.header.hd.so").is_some());
    }

    #[test]
    fn try_parse_rejects_unknown_fields() {
        assert!(BamHeaderFieldExecutor::try_parse("bam.header.hd.go").is_none());
        assert!(BamHeaderFieldExecutor::try_parse("bam.header.hd").is_none());
        assert!(BamHeaderFieldExecutor::try_parse("bam.header.rg.vn").is_none());
        // the old un-nested form no longer parses
        assert!(BamHeaderFieldExecutor::try_parse("bam.hd.vn").is_none());
    }
}
