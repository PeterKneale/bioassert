use crate::core::BioAssertError;
use crate::fastq::functions::scan;
use crate::suggest::Suggestion;
use crate::suggest::provider::SuggestionProvider;
use std::path::Path;

/// Suggests the default check for a FASTQ file (`.fastq`, `.fq`, optionally `.gz`): that it
/// parses as well-formed records. Emitted whether or not the file currently passes, since a
/// malformed read set is never the expectation.
pub struct FastqProvider;

impl SuggestionProvider for FastqProvider {
    fn name(&self) -> &'static str {
        "fastq"
    }

    fn handles(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let stem = name.strip_suffix(".gz").unwrap_or(&name);
        stem.ends_with(".fastq") || stem.ends_with(".fq")
    }

    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError> {
        scan(path)?;
        Ok(vec![Suggestion::new(
            path.to_string_lossy().as_ref(),
            "fastq.valid",
            "eq",
            "true",
            Some("every record is well formed"),
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_fastq_extensions() {
        assert!(FastqProvider.handles(Path::new("a.fastq")));
        assert!(FastqProvider.handles(Path::new("a.fq")));
        assert!(FastqProvider.handles(Path::new("a.fastq.gz")));
        assert!(FastqProvider.handles(Path::new("a.fq.gz")));
        assert!(FastqProvider.handles(Path::new("A.FASTQ.GZ")));
        assert!(!FastqProvider.handles(Path::new("a.fasta")));
        assert!(!FastqProvider.handles(Path::new("a.gz")));
        assert!(!FastqProvider.handles(Path::new("fastq")));
    }

    #[test]
    fn suggests_valid_check() {
        let suggestions = FastqProvider
            .suggest(Path::new("tests/data/sample.fastq"))
            .unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].metric, "fastq.valid");
        assert_eq!(suggestions[0].expected, "true");
    }

    #[test]
    fn suggests_valid_check_for_invalid_file() {
        let suggestions = FastqProvider
            .suggest(Path::new("tests/data/truncated.fastq"))
            .unwrap();
        assert_eq!(suggestions[0].metric, "fastq.valid");
    }

    #[test]
    fn errors_on_missing_file() {
        assert!(
            FastqProvider
                .suggest(Path::new("tests/data/missing.fastq"))
                .is_err()
        );
    }
}
