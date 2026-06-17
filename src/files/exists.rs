use std::path::PathBuf;
use crate::assertions::Value;

pub fn exists(file: &PathBuf) -> std::io::Result<Value> {
    Ok(Value::BooleanValue(file.exists() && file.is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn returns_true_when_file_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("files.txt");
        File::create(&file_path).unwrap();

        assert_eq!(exists(&file_path).unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn returns_false_when_file_does_not_exist() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("files.txt");

        assert_eq!(exists(&file_path).unwrap(), Value::BooleanValue(false));
    }
}
