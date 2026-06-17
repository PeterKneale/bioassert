use super::MetricExecutor;
use crate::assertions::{parse_comparator, parse_integer, Value};
use crate::parser::Assertion;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

pub struct FileLinesExecutor;

pub fn count_lines(file: &Path) -> std::io::Result<Value> {
    let count = BufReader::new(File::open(file)?).lines().count();
    Ok(Value::IntegerValue(count as u64))
}

impl MetricExecutor for FileLinesExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        (metric == "file.lines").then_some(Self)
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = count_lines(&file)?;
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
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn returns_zero_for_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        File::create(&path).unwrap();
        assert_eq!(count_lines(&path).unwrap(), Value::IntegerValue(0));
    }

    #[test]
    fn returns_correct_line_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"line1\nline2\nline3\n").unwrap();
        assert_eq!(count_lines(&path).unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn counts_line_without_trailing_newline() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"line1\nline2").unwrap();
        assert_eq!(count_lines(&path).unwrap(), Value::IntegerValue(2));
    }
}
