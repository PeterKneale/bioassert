use crate::values::{BytesValue, Value};
use std::path::Path;

pub fn size(file: &Path) -> std::io::Result<Value> {
    Ok(BytesValue(file.metadata()?.len()))
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
        assert_eq!(size(&path).unwrap(), BytesValue(0));
    }

    #[test]
    fn returns_byte_count_for_file_with_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello").unwrap();
        assert_eq!(size(&path).unwrap(), BytesValue(5));
    }

    #[test]
    fn returns_error_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        assert!(size(&path).is_err());
    }
}
