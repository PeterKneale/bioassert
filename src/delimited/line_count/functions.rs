use crate::core::{FileError, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn line_count(file: &Path) -> Result<Value, FileError> {
    let count = BufReader::new(File::open(file).map_err(|e| FileError::new(file, e))?)
        .lines()
        .count();
    Ok(Value::IntegerValue(count as u64))
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
