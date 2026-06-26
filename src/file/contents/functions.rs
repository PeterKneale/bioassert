use crate::core::FileError;
use std::fs;
use std::path::Path;

/// Reads the file body as a UTF-8 string. An I/O failure (a missing file) or non-UTF-8
/// content both map to a [`FileError`], so a binary file handed to a text metric reports
/// ERROR rather than misbehaving.
pub fn read_contents(file: &Path) -> Result<String, FileError> {
    fs::read_to_string(file).map_err(|e| FileError::new(file, e))
}

/// A bounded summary of the body for the report's `got` field. The comparison runs against
/// the full body, but the rendered `actual` is only the byte length, so a large or
/// sensitive file is never echoed into the report.
pub fn summarize(body: &str) -> String {
    format!("{} bytes", body.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_file(contents: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(contents).unwrap();
        f
    }

    #[test]
    fn reads_a_text_body() {
        let f = temp_file(b"line one\nline two\n");
        assert_eq!(read_contents(f.path()).unwrap(), "line one\nline two\n");
    }

    #[test]
    fn non_utf8_is_an_error() {
        // 0xFF is not valid UTF-8, so the read fails rather than returning garbage
        let f = temp_file(&[0x00, 0xFF, 0xFE]);
        assert!(read_contents(f.path()).is_err());
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(read_contents(Path::new("does/not/exist.log")).is_err());
    }

    #[test]
    fn summary_reports_byte_length_not_content() {
        assert_eq!(summarize("hello"), "5 bytes");
        assert_eq!(summarize(""), "0 bytes");
    }
}
