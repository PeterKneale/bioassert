use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use crate::assertions::parse_comparator;
use crate::parser::Assertion;
use std::path::PathBuf;
use super::MetricExecutor;

pub struct DelimitedCellExecutor {
    pub delimiter: char,
    pub line: usize,
    pub col: usize,
}

fn cell(file: &Path, delimiter: char, line: usize, column: usize) -> io::Result<String> {
    let reader = io::BufReader::new(File::open(file)?);
    let raw = reader
        .lines()
        .nth(line - 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("line {} not found", line)))??;
    super::parse_fields(&raw, delimiter)
        .into_iter()
        .nth(column - 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("column {} not found", column)))
}

fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 {
        let b = s.as_bytes();
        if (b[0] == b'\'' && b[s.len() - 1] == b'\'')
            || (b[0] == b'"' && b[s.len() - 1] == b'"')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

impl MetricExecutor for DelimitedCellExecutor {
    fn execute(self, assertion: Assertion) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected_str = strip_quotes(&assertion.expected).to_string();
        let actual = cell(&file, self.delimiter, self.line, self.col)?;
        let result = comparator.compare_string(&actual, &expected_str)?;
        let message = format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, expected_str, actual
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
    fn returns_csv_cell_value() {
        let f = temp_file("name,age,city\nAlice,30,New York\n");
        assert_eq!(cell(f.path(), ',', 1, 1).unwrap(), "name");
        assert_eq!(cell(f.path(), ',', 2, 1).unwrap(), "Alice");
        assert_eq!(cell(f.path(), ',', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn returns_tsv_cell_value() {
        let f = temp_file("name\tage\tcity\nAlice\t30\tNew York\n");
        assert_eq!(cell(f.path(), '\t', 1, 1).unwrap(), "name");
        assert_eq!(cell(f.path(), '\t', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn returns_psv_cell_value() {
        let f = temp_file("name|age|city\nAlice|30|New York\n");
        assert_eq!(cell(f.path(), '|', 1, 1).unwrap(), "name");
        assert_eq!(cell(f.path(), '|', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn strips_double_quoted_field() {
        let f = temp_file("name,age,city\nAlice,30,\"New York\"\n");
        assert_eq!(cell(f.path(), ',', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn errors_on_missing_line() {
        let f = temp_file("name,age\n");
        assert!(cell(f.path(), ',', 99, 1).is_err());
    }

    #[test]
    fn errors_on_missing_column() {
        let f = temp_file("name,age\n");
        assert!(cell(f.path(), ',', 1, 99).is_err());
    }

    #[test]
    fn strip_quotes_removes_double_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn strip_quotes_removes_single_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_unquoted_string() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_short_string() {
        assert_eq!(strip_quotes("a"), "a");
        assert_eq!(strip_quotes(""), "");
    }
}
