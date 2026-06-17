use crate::assertions::{parse_comparator, parse_integer, BioAssertError, FileError};
use crate::metrics::MetricExecutor;
use crate::parser::Assertion;
use std::path::PathBuf;

pub struct FileLinesExecutor;

impl MetricExecutor for FileLinesExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.lines").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = super::functions::count_lines(&file).map_err(|e| FileError::new(&file, e))?;
        let result = comparator.compare(&actual, &expected);
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected, actual
        );
        Ok((result, message))
    }
}
