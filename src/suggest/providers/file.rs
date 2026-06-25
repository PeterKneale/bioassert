use crate::core::{BioAssertError, Value};
use crate::file::size::functions::get_file_size;
use crate::suggest::Suggestion;
use crate::suggest::provider::SuggestionProvider;
use std::path::Path;

/// Suggests the universal file checks that apply to any path: presence, and (when the file is
/// present) a broad non-empty floor. The non-empty floor is the fixed `gt 0B` rather than a
/// banded byte count, because the human-readable byte rendering is lossy above 1KB; `0B`
/// always round-trips.
pub struct FileProvider;

impl SuggestionProvider for FileProvider {
    fn name(&self) -> &'static str {
        "file"
    }

    fn handles(&self, _path: &Path) -> bool {
        true
    }

    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError> {
        let resource = path.to_string_lossy();
        let mut suggestions = vec![Suggestion::new(
            resource.as_ref(),
            "file.exists",
            "eq",
            "true",
            Some("file is present"),
        )];
        // A missing file has no current size to anchor on, and `file.exists` already covers
        // it, so emit the size floor only when the size function can read the file. The
        // emitted value is a fixed `0B` rendered through the single `Value::Display` path.
        if get_file_size(path).is_ok() {
            suggestions.push(Suggestion::new(
                resource.as_ref(),
                "file.size",
                "gt",
                Value::BytesValue(0).to_string(),
                Some("file is not empty"),
            ));
        }
        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn present_file_gets_exists_and_size() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.txt");
        fs::write(&path, b"hello").unwrap();

        let suggestions = FileProvider.suggest(&path).unwrap();
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].metric, "file.exists");
        assert_eq!(suggestions[0].comparator, "eq");
        assert_eq!(suggestions[0].expected, "true");
        assert_eq!(suggestions[1].metric, "file.size");
        assert_eq!(suggestions[1].comparator, "gt");
        assert_eq!(suggestions[1].expected, "0B");
    }

    #[test]
    fn missing_file_gets_only_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.txt");

        let suggestions = FileProvider.suggest(&path).unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].metric, "file.exists");
    }

    #[test]
    fn handles_any_path() {
        assert!(FileProvider.handles(Path::new("anything.xyz")));
        assert!(FileProvider.handles(Path::new("no_extension")));
    }
}
