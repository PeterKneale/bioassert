use super::functions::{compressed, compression};
use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};

/// `file.compression` — classifies the file's compression or archive format from its
/// leading magic bytes (`none`, `gzip`, `bgzf`, `bzip2`, `xz`, `zstd`, `zip`) and compares
/// the label as a string. `bgzf` is reported in preference to `gzip` for block-gzip files,
/// so `if reads.gz file.compression eq bgzf` gates a samtools or tabix step correctly.
pub struct FileCompressionExecutor;

impl AssertionExecutor for FileCompressionExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.compression").then_some(Self)
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        // strip quotes from the expected label/regex, as the other string executors do
        let expected = crate::core::strip_quotes(&request.expected);
        let actual = compression(request.path())?;
        let success = request
            .comparator
            .compare_string(&actual.to_string(), expected)?;
        Ok(AssertionExecutionResult { success, actual })
    }
}

/// `file.compressed` — boolean, true when the file carries any recognised compression or
/// archive magic. A convenient guard input (`if data.gz file.compressed eq true`).
pub struct FileCompressedExecutor;

impl AssertionExecutor for FileCompressedExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.compressed").then_some(Self)
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = Value::from_boolean(&request.expected)?;
        let actual = compressed(request.path())?;
        let success = request.comparator.compare(&actual, &expected);
        Ok(AssertionExecutionResult { success, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_try_parse_matches_only_its_metric() {
        assert!(FileCompressionExecutor::try_parse("file.compression").is_some());
        assert!(FileCompressionExecutor::try_parse("file.compressed").is_none());
        assert!(FileCompressionExecutor::try_parse("file.size").is_none());
    }

    #[test]
    fn compressed_try_parse_matches_only_its_metric() {
        assert!(FileCompressedExecutor::try_parse("file.compressed").is_some());
        assert!(FileCompressedExecutor::try_parse("file.compression").is_none());
        assert!(FileCompressedExecutor::try_parse("file.exists").is_none());
    }
}
