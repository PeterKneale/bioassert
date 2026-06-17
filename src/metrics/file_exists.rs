use super::MetricExecutor;
use crate::assertions::{parse_boolean, parse_comparator, Value};
use crate::parser::Assertion;
use std::path::PathBuf;

pub struct FileExistsExecutor;

fn exists(file: &PathBuf) -> std::io::Result<Value> {
    Ok(Value::BooleanValue(file.exists() && file.is_file()))
}

impl MetricExecutor for FileExistsExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.exists").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_boolean(assertion.expected.as_str())?;
        let actual = exists(&file)?;
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
    use tempfile::tempdir;

    #[test]
    fn returns_true_when_file_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        File::create(&path).unwrap();
        assert_eq!(exists(&path).unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn returns_false_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        assert_eq!(exists(&path).unwrap(), Value::BooleanValue(false));
    }
}
