use super::delimited_utils::{delimiter_for_prefix, parse_fields};
use super::MetricExecutor;
use crate::assertions::{parse_comparator, parse_integer, BioAssertError, FileError, Value};
use crate::parser::Assertion;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;

pub struct DelimitedColumnCountExecutor {
    pub delimiter: char,
}

fn column_count(file: &Path, delimiter: char) -> io::Result<Value> {
    let mut reader = io::BufReader::new(File::open(file)?);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let count = parse_fields(first_line.trim_end_matches(['\n', '\r']), delimiter).len();
    Ok(Value::IntegerValue(count as u64))
}

impl MetricExecutor for DelimitedColumnCountExecutor {
    fn try_parse(metric: &str) -> Option<Self> {
        let (prefix, rest) = metric.split_once('.')?;
        let delimiter = delimiter_for_prefix(prefix)?;
        (rest == "columns.count").then_some(Self { delimiter })
    }

    fn execute(self, assertion: Assertion) -> Result<(bool, String), BioAssertError> {
        let file = PathBuf::from(&assertion.file);
        let comparator = parse_comparator(assertion.comparator.as_str())?;
        let expected = parse_integer(assertion.expected.as_str())?;
        let actual = column_count(&file, self.delimiter).map_err(|e| FileError::new(&file, e))?;
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
    fn counts_csv_header_fields() {
        let f = temp_file("name,age,city\nAlice,30,New York\n");
        assert_eq!(column_count(f.path(), ',').unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn counts_tsv_header_fields() {
        let f = temp_file("name\tage\tcity\nAlice\t30\tNew York\n");
        assert_eq!(column_count(f.path(), '\t').unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn counts_psv_header_fields() {
        let f = temp_file("name|age|city\nAlice|30|New York\n");
        assert_eq!(column_count(f.path(), '|').unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn counts_single_column() {
        let f = temp_file("name\nAlice\n");
        assert_eq!(column_count(f.path(), ',').unwrap(), Value::IntegerValue(1));
    }

    #[test]
    fn try_parse_csv_columns_count() {
        assert!(matches!(
            DelimitedColumnCountExecutor::try_parse("csv.columns.count"),
            Some(DelimitedColumnCountExecutor { delimiter: ',' })
        ));
    }

    #[test]
    fn try_parse_tsv_columns_count() {
        assert!(matches!(
            DelimitedColumnCountExecutor::try_parse("tsv.columns.count"),
            Some(DelimitedColumnCountExecutor { delimiter: '\t' })
        ));
    }

    #[test]
    fn try_parse_rejects_unknown_prefix() {
        assert!(DelimitedColumnCountExecutor::try_parse("dsv.columns.count").is_none());
    }

    #[test]
    fn try_parse_rejects_wrong_suffix() {
        assert!(DelimitedColumnCountExecutor::try_parse("csv.lines.count").is_none());
    }
}
