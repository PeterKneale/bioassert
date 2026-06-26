use super::functions;
use crate::core::{
    AssertionExecutionResult, AssertionExecutor, AssertionRequest, BioAssertError, Value,
};

/// Asserts on the file's whole body as a string.
///
/// `file.contents` reads the file as UTF-8 text and compares the entire body with the
/// string comparators (`eq`, `ne`, `starts`, `ends`, `contains`, `matches`), each of which
/// may carry the `not` modifier. `contains` is a substring search over the whole body and
/// `matches` is a regex search over it, so `file.contents not contains 'Exception'` checks
/// that a word appears nowhere in the file. It is the file-backed twin of `text.value`.
///
/// The body is read into memory, so this metric is for log-sized and config-sized text,
/// not multi-gigabyte genomes. The reported `actual` is a bounded summary (the body's byte
/// length), never the content, so a large or sensitive file is not echoed into the report.
pub struct FileContentsExecutor;

impl AssertionExecutor for FileContentsExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.contents").then_some(Self)
    }

    fn execute(
        self,
        request: &AssertionRequest,
    ) -> Result<AssertionExecutionResult, BioAssertError> {
        let expected = crate::core::strip_quotes(&request.expected);
        let body = functions::read_contents(request.path())?;
        let success = request.comparator.compare_string(&body, expected)?;
        Ok(AssertionExecutionResult {
            success,
            actual: Value::StringValue(functions::summarize(&body)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AssertionExecutor;

    #[test]
    fn try_parse_matches_only_file_contents() {
        assert!(FileContentsExecutor::try_parse("file.contents").is_some());
        assert!(FileContentsExecutor::try_parse("file.lines").is_none());
        assert!(FileContentsExecutor::try_parse("file.content").is_none());
    }
}
