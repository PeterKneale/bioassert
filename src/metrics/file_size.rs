use super::MetricExecutor;
use crate::assertions::{parse_bytes, parse_comparator, BioAssertError, BytesValue, FileError, Value};
use crate::parser::Assertion;
use std::path::Path;
use std::path::PathBuf;

pub struct FileSizeExecutor;

fn size(file: &Path) -> std::io::Result<Value> {
    Ok(BytesValue(file.metadata()?.len()))
}

impl MetricExecutor for FileSizeExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.size").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_bytes(assertion.expected.as_str())?;
        let actual = size(&file).map_err(|e| FileError::new(&file, e))?;
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
    fn returns_zero_when_file_is_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        File::create(&path).unwrap();
        assert_eq!(size(&path).unwrap(), BytesValue(0));
    }

    #[test]
    fn returns_byte_count_for_file_with_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello").unwrap();
        assert_eq!(size(&path).unwrap(), BytesValue(5));
    }

    #[test]
    fn returns_error_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        assert!(size(&path).is_err());
    }
}
