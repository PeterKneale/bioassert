use super::MetricExecutor;
use crate::assertions::{parse_comparator, parse_integer, Value};
use crate::parser::Assertion;
use std::path::Path;
use std::path::PathBuf;

pub struct DelimitedLineCountExecutor;

fn line_count(file: &Path) -> std::io::Result<Value> {
    super::file_lines::count_lines(file)
}

impl MetricExecutor for DelimitedLineCountExecutor {
    fn execute(self, assertion: Assertion) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = line_count(&file)?;
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
    use tempfile::NamedTempFile;

    fn temp_file(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f
    }

    #[test]
    fn counts_all_lines_including_header() {
        let f = temp_file("name,age,city\nAlice,30,New York\nBob,25,LA\n");
        assert_eq!(line_count(f.path()).unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn counts_lines_in_tsv() {
        let f = temp_file("name\tage\nAlice\t30\n");
        assert_eq!(line_count(f.path()).unwrap(), Value::IntegerValue(2));
    }

    #[test]
    fn returns_zero_for_empty_file() {
        let f = temp_file("");
        assert_eq!(line_count(f.path()).unwrap(), Value::IntegerValue(0));
    }
}
