use crate::file_error::FileError;
use crate::values::Value;
use std::path::Path;

pub fn empty(file: &Path) -> Result<Value, FileError> {
    file.metadata()
        .map(|m| Value::BooleanValue(m.len() == 0))
        .map_err(|e| FileError::new(file, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn returns_true_when_file_is_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        File::create(&path).unwrap();
        assert_eq!(empty(&path).unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn returns_false_when_file_has_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello").unwrap();
        assert_eq!(empty(&path).unwrap(), Value::BooleanValue(false));
    }
}
