use bioassert_core::{BytesValue, FileError, Value};
use std::path::Path;

pub fn get_file_size(file: &Path) -> Result<Value, FileError> {
    file.metadata()
        .map(|m| BytesValue(m.len()))
        .map_err(|e| FileError::new(file, e))
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
        assert_eq!(get_file_size(&path).unwrap(), BytesValue(0));
    }

    #[test]
    fn returns_byte_count_for_file_with_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello").unwrap();
        assert_eq!(get_file_size(&path).unwrap(), BytesValue(5));
    }

    #[test]
    fn returns_error_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        assert!(get_file_size(&path).is_err());
    }
}
