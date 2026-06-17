use std::path::Path;
use crate::assertions::Value;

pub fn empty(file: &Path) -> std::io::Result<Value> {
    Ok(Value::BooleanValue(file.metadata()?.len() == 0))
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
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        assert_eq!(empty(&file_path).unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn returns_false_when_file_has_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"hello").unwrap();

        assert_eq!(empty(&file_path).unwrap(), Value::BooleanValue(false));
    }
}
