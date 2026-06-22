use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, FileError, Value};
use crate::fasta::functions;
use std::io;
use std::path::Path;

/// Reads a per-record field by index: `fasta.seq.<n>.name`, `fasta.seq.<n>.description`, or
/// `fasta.seq.<n>.length`. Names and descriptions compare as strings; length compares
/// numerically. Errors when the record index is out of range, or (for `description`) when the
/// record has no description.
pub struct FastaSequenceFieldExecutor {
    index: usize,
    field: Field,
}

#[derive(Clone, Copy)]
enum Field {
    Name,
    Description,
    Length,
}

impl AssertionExecutor for FastaSequenceFieldExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let (n, field) = match parts.as_slice() {
            ["fasta", "seq", n, "name"] => (n, Field::Name),
            ["fasta", "seq", n, "description"] => (n, Field::Description),
            ["fasta", "seq", n, "length"] => (n, Field::Length),
            _ => return None,
        };
        Some(Self { index: n.parse().ok()?, field })
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let records = functions::read_records(&request.file)?;
        match self.field {
            Field::Length => {
                let expected = Value::from_integer(&request.expected)?;
                let length = functions::record_length(&records, self.index)
                    .ok_or_else(|| out_of_range(&request.file, self.index))?;
                let actual = Value::IntegerValue(length);
                let success = request.comparator.compare(&actual, &expected);
                Ok(AssertionExecutionResult { success, actual })
            }
            Field::Name => {
                let expected = crate::core::strip_quotes(&request.expected).to_string();
                let actual = functions::record_name(&records, self.index)
                    .ok_or_else(|| out_of_range(&request.file, self.index))?;
                let success = request.comparator.compare_string(&actual, &expected)?;
                Ok(AssertionExecutionResult { success, actual: Value::StringValue(actual) })
            }
            Field::Description => {
                // Distinguish an out-of-range record from a present-but-undescribed one so the
                // error message is meaningful; both still error, per the spec semantics.
                if !functions::record_present(&records, self.index) {
                    return Err(out_of_range(&request.file, self.index));
                }
                let expected = crate::core::strip_quotes(&request.expected).to_string();
                let actual = functions::record_description(&records, self.index)
                    .filter(|d| !d.is_empty())
                    .ok_or_else(|| {
                        field_error(&request.file, format!("record {} has no description", self.index))
                    })?;
                let success = request.comparator.compare_string(&actual, &expected)?;
                Ok(AssertionExecutionResult { success, actual: Value::StringValue(actual) })
            }
        }
    }
}

/// Tests presence of a record or its description, returning a boolean that never errors on
/// absence: `fasta.seq.<n>.present` and `fasta.seq.<n>.description.present`.
pub struct FastaSequencePresentExecutor {
    index: usize,
    description: bool,
}

impl AssertionExecutor for FastaSequencePresentExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        let (n, description) = match parts.as_slice() {
            ["fasta", "seq", n, "present"] => (n, false),
            ["fasta", "seq", n, "description", "present"] => (n, true),
            _ => return None,
        };
        Some(Self { index: n.parse().ok()?, description })
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let records = functions::read_records(&request.file)?;
        let present = if self.description {
            functions::record_description(&records, self.index).is_some_and(|d| !d.is_empty())
        } else {
            functions::record_present(&records, self.index)
        };
        let actual = Value::BooleanValue(present);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

/// Error for a record index that does not exist.
fn out_of_range(file: &Path, index: usize) -> BioAssertError {
    field_error(file, format!("record {index} not found"))
}

/// Builds an `InvalidInput` [`FileError`] carrying a per-record diagnostic message.
fn field_error(file: &Path, message: String) -> BioAssertError {
    FileError::new(file, io::Error::new(io::ErrorKind::InvalidInput, message)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_field_accepts_known_fields() {
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.0.name").is_some());
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.12.description").is_some());
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.3.length").is_some());
    }

    #[test]
    fn try_parse_field_rejects() {
        // non-numeric index, unknown field, the present shapes, and wrong arity
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.x.name").is_none());
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.0.gc").is_none());
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.0.present").is_none());
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.0.description.present").is_none());
        assert!(FastaSequenceFieldExecutor::try_parse("fasta.seq.0").is_none());
    }

    #[test]
    fn try_parse_present_accepts() {
        assert!(FastaSequencePresentExecutor::try_parse("fasta.seq.0.present").is_some());
        assert!(FastaSequencePresentExecutor::try_parse("fasta.seq.0.description.present").is_some());
    }

    #[test]
    fn try_parse_present_rejects() {
        assert!(FastaSequencePresentExecutor::try_parse("fasta.seq.0.name.present").is_none());
        assert!(FastaSequencePresentExecutor::try_parse("fasta.seq.x.present").is_none());
        assert!(FastaSequencePresentExecutor::try_parse("fasta.seq.0.name").is_none());
    }
}
