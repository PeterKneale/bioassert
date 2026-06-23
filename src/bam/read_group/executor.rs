use crate::bam::functions;
use crate::core::{AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, FileError, Value};
use std::io;

/// Reads a read-group tag value by index: `bam.header.rg.<n>.<tag>` (e.g.
/// `bam.header.rg.0.sm`). `id` resolves to the read-group identifier; other 2-letter tags
/// resolve to the record's fields. Errors when the index is out of range or the tag is not set.
pub struct BamReadGroupTagExecutor {
    index: usize,
    tag: String,
}

impl AssertionExecutor for BamReadGroupTagExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            ["bam", "header", "rg", n, tag] if is_tag(tag) => Some(Self {
                index: n.parse().ok()?,
                tag: tag.to_string(),
            }),
            _ => None,
        }
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected).to_string();
        let header = functions::read_header(request.path())?;
        let actual = functions::read_group_tag(&header, self.index, &self.tag).ok_or_else(|| {
            FileError::new(
                request.path(),
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("read group {} tag {} not found", self.index, self.tag),
                ),
            )
        })?;
        let success = request.comparator.compare_string(&actual, &expected)?;
        Ok(AssertionExecutionResult { success, actual: Value::StringValue(actual) })
    }
}

/// Tests presence of a read group or one of its tags, returning a boolean that never errors
/// on absence: `bam.header.rg.<n>.present` and `bam.header.rg.<n>.<tag>.present`.
pub struct BamReadGroupPresentExecutor {
    index: usize,
    tag: Option<String>,
}

impl AssertionExecutor for BamReadGroupPresentExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            ["bam", "header", "rg", n, "present"] => Some(Self {
                index: n.parse().ok()?,
                tag: None,
            }),
            ["bam", "header", "rg", n, tag, "present"] if is_tag(tag) => Some(Self {
                index: n.parse().ok()?,
                tag: Some(tag.to_string()),
            }),
            _ => None,
        }
    }

    fn execute(self, request: &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let header = functions::read_header(request.path())?;
        let present = match &self.tag {
            None => functions::read_group_present(&header, self.index),
            Some(tag) => functions::read_group_tag(&header, self.index, tag).is_some(),
        };
        let actual = Value::BooleanValue(present);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

/// A read-group metric tag is exactly two ASCII letters (e.g. `sm`, `pu`). This keeps the
/// 7-letter `present` suffix out of the tag position so the present executor owns it.
fn is_tag(segment: &str) -> bool {
    segment.len() == 2 && segment.bytes().all(|b| b.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_tag_value() {
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.0.sm").is_some());
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.12.pu").is_some());
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.0.id").is_some());
    }

    #[test]
    fn try_parse_tag_value_rejects() {
        // `present` is not a tag, three-letter segments are not tags, non-numeric index.
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.0.present").is_none());
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.0.foo").is_none());
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.x.sm").is_none());
        assert!(BamReadGroupTagExecutor::try_parse("bam.header.rg.0.sm.present").is_none());
        // the old un-nested form no longer parses
        assert!(BamReadGroupTagExecutor::try_parse("bam.rg.0.sm").is_none());
    }

    #[test]
    fn try_parse_present() {
        assert!(BamReadGroupPresentExecutor::try_parse("bam.header.rg.0.present").is_some());
        assert!(BamReadGroupPresentExecutor::try_parse("bam.header.rg.0.sm.present").is_some());
    }

    #[test]
    fn try_parse_present_rejects() {
        assert!(BamReadGroupPresentExecutor::try_parse("bam.header.rg.0.sm").is_none());
        assert!(BamReadGroupPresentExecutor::try_parse("bam.header.rg.0.foo.present").is_none());
        assert!(BamReadGroupPresentExecutor::try_parse("bam.header.rg.x.present").is_none());
    }
}
