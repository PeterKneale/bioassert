use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use crate::assertions::Value;

pub fn lines(file: &Path) -> std::io::Result<Value> {
    let count = BufReader::new(File::open(file)?).lines().count();
    Ok(Value::IntegerValue(count as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn returns_zero_for_empty_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        assert_eq!(lines(&file_path).unwrap(), Value::IntegerValue(0));
    }

    #[test]
    fn returns_correct_line_count() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"line1\nline2\nline3\n").unwrap();

        assert_eq!(lines(&file_path).unwrap(), Value::IntegerValue(3));
    }

    #[test]
    fn counts_line_without_trailing_newline() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"line1\nline2").unwrap();

        assert_eq!(lines(&file_path).unwrap(), Value::IntegerValue(2));
    }
}
