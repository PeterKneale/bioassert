use crate::bam::functions;
use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, FileError, Value,
};
use std::io;

/// Reads a `@PG` program tag value by index: `bam.header.pg.<n>.<tag>` (e.g.
/// `bam.header.pg.0.cl`). `id` resolves to the program identifier; other 2-letter tags
/// (`pn`, `pp`, `vn`, `cl`, ...) resolve to the record's fields. Errors when the index is
/// out of range or the tag is not set. The `pg` segment and its SAM tags are matched
/// case-insensitively, since they are uppercase in the SAM header itself.
pub struct BamProgramTagExecutor {
    index: usize,
    tag: String,
}

impl AssertionExecutor for BamProgramTagExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            ["bam", "header", pg, n, tag] if pg.eq_ignore_ascii_case("pg") && is_tag(tag) => {
                Some(Self {
                    index: n.parse().ok()?,
                    tag: tag.to_string(),
                })
            }
            _ => None,
        }
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected).to_string();
        let header = functions::read_header(request.path())?;
        let actual = functions::program_tag(&header, self.index, &self.tag).ok_or_else(|| {
            FileError::new(
                request.path(),
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("program {} tag {} not found", self.index, self.tag),
                ),
            )
        })?;
        let success = request.comparator.compare_string(&actual, &expected)?;
        Ok(AssertionExecutionResult {
            success,
            actual: Value::StringValue(actual),
        })
    }
}

/// Tests presence of a `@PG` program or one of its tags, returning a boolean that never errors
/// on absence: `bam.header.pg.<n>.present` and `bam.header.pg.<n>.<tag>.present`.
pub struct BamProgramPresentExecutor {
    index: usize,
    tag: Option<String>,
}

impl AssertionExecutor for BamProgramPresentExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let parts: Vec<&str> = metric.split('.').collect();
        match parts.as_slice() {
            ["bam", "header", pg, n, "present"] if pg.eq_ignore_ascii_case("pg") => Some(Self {
                index: n.parse().ok()?,
                tag: None,
            }),
            ["bam", "header", pg, n, tag, "present"]
                if pg.eq_ignore_ascii_case("pg") && is_tag(tag) =>
            {
                Some(Self {
                    index: n.parse().ok()?,
                    tag: Some(tag.to_string()),
                })
            }
            _ => None,
        }
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let header = functions::read_header(request.path())?;
        let present = match &self.tag {
            None => functions::program_present(&header, self.index),
            Some(tag) => functions::program_tag(&header, self.index, tag).is_some(),
        };
        let actual = Value::BooleanValue(present);
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

/// A program metric tag is exactly two ASCII letters (e.g. `cl`, `pp`). This keeps the
/// 7-letter `present` suffix out of the tag position so the present executor owns it.
fn is_tag(segment: &str) -> bool {
    segment.len() == 2 && segment.bytes().all(|b| b.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_tag_value() {
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.0.id").is_some());
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.1.cl").is_some());
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.0.pp").is_some());
        // the pg segment and the tag are case-insensitive (uppercase in the file).
        assert!(BamProgramTagExecutor::try_parse("bam.header.PG.0.ID").is_some());
        assert!(BamProgramTagExecutor::try_parse("bam.header.Pg.1.Cl").is_some());
    }

    #[test]
    fn try_parse_tag_value_rejects() {
        // `present` is not a tag, three-letter segments are not tags, non-numeric index.
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.0.present").is_none());
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.0.foo").is_none());
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.x.cl").is_none());
        assert!(BamProgramTagExecutor::try_parse("bam.header.pg.0.cl.present").is_none());
        // a different record type must not match.
        assert!(BamProgramTagExecutor::try_parse("bam.header.rg.0.sm").is_none());
    }

    #[test]
    fn try_parse_present() {
        assert!(BamProgramPresentExecutor::try_parse("bam.header.pg.0.present").is_some());
        assert!(BamProgramPresentExecutor::try_parse("bam.header.pg.1.pp.present").is_some());
        assert!(BamProgramPresentExecutor::try_parse("bam.header.PG.1.PP.present").is_some());
    }

    #[test]
    fn try_parse_present_rejects() {
        assert!(BamProgramPresentExecutor::try_parse("bam.header.pg.0.cl").is_none());
        assert!(BamProgramPresentExecutor::try_parse("bam.header.pg.0.foo.present").is_none());
        assert!(BamProgramPresentExecutor::try_parse("bam.header.pg.x.present").is_none());
    }
}
