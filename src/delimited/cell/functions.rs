use crate::core::FileError;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub fn cell(file: &Path, delimiter: char, line: usize, column: usize) -> Result<String, FileError> {
    let reader = io::BufReader::new(File::open(file).map_err(|e| FileError::new(file, e))?);
    let raw = reader
        .lines()
        .nth(line - 1)
        .ok_or_else(|| {
            FileError::new(
                file,
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("line {} not found", line),
                ),
            )
        })?
        .map_err(|e| FileError::new(file, e))?;
    super::super::functions::parse_fields(&raw, delimiter)
        .into_iter()
        .nth(column - 1)
        .ok_or_else(|| {
            FileError::new(
                file,
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("column {} not found", column),
                ),
            )
        })
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
}
