use super::MetricExecutor;
use crate::assertions::{parse_boolean, parse_comparator, BioAssertError, FileError, Value};
use crate::parser::Assertion;
use std::path::Path;
use std::path::PathBuf;

pub struct FileEmptyExecutor;

fn empty(file: &Path) -> std::io::Result<Value> {
    Ok(Value::BooleanValue(file.metadata()?.len() == 0))
}

impl MetricExecutor for FileEmptyExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.empty").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_boolean(assertion.expected.as_str())?;
        let actual = empty(&file).map_err(|e| FileError::new(&file, e))?;
        let result = comparator.compare(&actual, &expected);
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected, actual
        );
        Ok((result, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn returns_true_when_file_is_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        File::create(&path).unwrap();
        assert_eq!(empty(&path).unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn returns_false_when_file_has_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello").unwrap();
        assert_eq!(empty(&path).unwrap(), Value::BooleanValue(false));
    }
}
