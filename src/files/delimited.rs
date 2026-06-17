use std::io::{self, BufRead};
use std::fs::File;
use std::path::Path;
use crate::assertions::Value;
use crate::files::lines::count_lines;

pub fn column_count(file: &Path, delimiter: char) -> io::Result<Value> {
    let f = File::open(file)?;
    let mut reader = io::BufReader::new(f);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let count = parse_fields(first_line.trim_end_matches(['\n', '\r']), delimiter).len();
    Ok(Value::IntegerValue(count as u64))
}

pub fn line_count(file: &Path) -> io::Result<Value> {
    count_lines(file)
}

pub fn cell(file: &Path, delimiter: char, line: usize, column: usize) -> io::Result<String> {
    let f = File::open(file)?;
    let reader = io::BufReader::new(f);
    let raw = reader
        .lines()
        .nth(line - 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("line {} not found", line)))??;
    let fields = parse_fields(&raw, delimiter);
    fields
        .into_iter()
        .nth(column - 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("column {} not found", column)))
}

fn parse_fields(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in line.chars() {
        if in_quotes {
            if ch == quote_char {
                in_quotes = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
        } else if ch == delimiter {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn csv_file(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f
    }

    #[test]
    fn column_count_counts_header_fields() {
        let f = csv_file("name,age,city\nAlice,30,New York\n");
        assert_eq!(column_count(f.path(), ',').unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn line_count_counts_all_lines() {
        let f = csv_file("name,age,city\nAlice,30,New York\nBob,25,LA\n");
        assert_eq!(line_count(f.path()).unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn cell_returns_unquoted_value() {
        let f = csv_file("name,age,city\nAlice,30,New York\n");
        assert_eq!(cell(f.path(), ',', 1, 1).unwrap(), "name");
        assert_eq!(cell(f.path(), ',', 2, 1).unwrap(), "Alice");
        assert_eq!(cell(f.path(), ',', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn cell_strips_double_quotes() {
        let f = csv_file("name,age,city\nAlice,30,\"New York\"\n");
        assert_eq!(cell(f.path(), ',', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn parse_fields_splits_on_delimiter() {
        assert_eq!(parse_fields("a,b,c", ','), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_fields_handles_tab_delimiter() {
        assert_eq!(parse_fields("a\tb\tc", '\t'), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_fields_strips_quotes() {
        assert_eq!(parse_fields("\"hello world\",b", ','), vec!["hello world", "b"]);
    }

    #[test]
    fn column_count_counts_tsv_header_fields() {
        let f = csv_file("name\tage\tcity\nAlice\t30\tNew York\n");
        assert_eq!(column_count(f.path(), '\t').unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn cell_returns_tsv_value() {
        let f = csv_file("name\tage\tcity\nAlice\t30\tNew York\n");
        assert_eq!(cell(f.path(), '\t', 1, 1).unwrap(), "name");
        assert_eq!(cell(f.path(), '\t', 2, 3).unwrap(), "New York");
    }

    #[test]
    fn cell_errors_on_missing_line() {
        let f = csv_file("name,age\n");
        assert!(cell(f.path(), ',', 99, 1).is_err());
    }

    #[test]
    fn cell_errors_on_missing_column() {
        let f = csv_file("name,age\n");
        assert!(cell(f.path(), ',', 1, 99).is_err());
    }
}
