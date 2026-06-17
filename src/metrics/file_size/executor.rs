use crate::assertions::{parse_bytes, parse_comparator, BioAssertError, FileError};
use crate::metrics::MetricExecutor;
use crate::parser::Assertion;
use std::path::PathBuf;

pub struct FileSizeExecutor;

impl MetricExecutor for FileSizeExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.size").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_bytes(assertion.expected.as_str())?;
        let actual = super::functions::size(&file).map_err(|e| FileError::new(&file, e))?;
        let result = comparator.compare(&actual, &expected);
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected, actual
        );
        Ok((result, message))
    }
}
