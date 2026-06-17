use crate::assertions::{parse_boolean, parse_comparator, BioAssertError, FileError};
use crate::metrics::MetricExecutor;
use crate::parser::Assertion;
use std::path::PathBuf;

pub struct FileEmptyExecutor;

impl MetricExecutor for FileEmptyExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.empty").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_boolean(assertion.expected.as_str())?;
        let actual = super::functions::empty(&file).map_err(|e| FileError::new(&file, e))?;
        let result = comparator.compare(&actual, &expected);
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected, actual
        );
        Ok((result, message))
    }
}
