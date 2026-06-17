use std::path::Path;
use crate::assertions::{BytesValue, Value};

pub fn size(file: &Path) -> Result<Value,std::io::Error> {
    if !file.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("file not found: {}",file.display())));
    }
    let bytes = file.metadata()?.len();
    Ok(BytesValue(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::{TempDir, tempdir};

    #[test]
    fn returns_zero_when_file_is_empty() {
        // arrange
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("files.txt");
        File::create(&file_path).unwrap();

        // act
        let result = size(&file_path);

        //assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BytesValue(0));
    }

    #[test]
    fn returns_5_when_file_contains_hello() {
        // arrange
        let dir: TempDir = tempdir().unwrap();
        let file_path: PathBuf = dir.path().join("files.txt");
        let mut buffer: File = File::create(&file_path).unwrap();
        buffer.write_all(b"hello").unwrap();

        // act
        let result = size(&file_path);

        //assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BytesValue(5));
    }
}
