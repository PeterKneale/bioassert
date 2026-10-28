use crate::core::BioAssertError;
use crate::fastq::functions::{QualityEncoding, scan};
use crate::suggest::Suggestion;
use crate::suggest::provider::SuggestionProvider;
use crate::suggest::suggestion::band;
use std::path::Path;

/// Suggests the default checks for a FASTQ file (`.fastq`, `.fq`, optionally `.gz`): that it
/// parses as well-formed records, and when it does, a +/- 50% band on the read count, the
/// current shortest and longest read as bounds, and the quality encoding when it could be
/// inferred. The validity check is emitted whether or not the file currently passes, since a
/// malformed read set is never the expectation; the rest need a complete scan.
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
        let resource = path.to_string_lossy();
        let scan = scan(path)?;
        let mut suggestions = vec![Suggestion::new(
            resource.as_ref(),
            "fastq.valid",
            "eq",
            "true",
            Some("every record is well formed"),
        )];
        if !scan.is_valid() {
            return Ok(suggestions);
        }
        let (lower, upper) = band(scan.reads);
        suggestions.extend([
            Suggestion::new(
                resource.as_ref(),
                "fastq.read.count",
                "gte",
                lower.to_string(),
                Some(&format!("read count within +/- 50% of {}", scan.reads)),
            ),
            Suggestion::new(
                resource.as_ref(),
                "fastq.read.count",
                "lte",
                upper.to_string(),
                None,
            ),
            Suggestion::new(
                resource.as_ref(),
                "fastq.read.length.min",
                "gte",
                scan.min_length.to_string(),
                Some("shortest read"),
            ),
            Suggestion::new(
                resource.as_ref(),
                "fastq.read.length.max",
                "lte",
                scan.max_length.to_string(),
                Some("longest read"),
            ),
        ]);
        let encoding = scan.encoding();
        if encoding != QualityEncoding::Ambiguous {
            suggestions.push(Suggestion::new(
                resource.as_ref(),
                "fastq.quality.encoding",
                "eq",
                encoding.as_str(),
                Some("inferred from the quality characters"),
            ));
        }
        Ok(suggestions)
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
    fn sample_fastq_suggests_full_set() {
        let suggestions = FastqProvider
            .suggest(Path::new("tests/data/sample.fastq"))
            .unwrap();
        let lines: Vec<(&str, &str, &str)> = suggestions
            .iter()
            .map(|s| (s.metric.as_str(), s.comparator, s.expected.as_str()))
            .collect();
        assert_eq!(
            lines,
            [
                ("fastq.valid", "eq", "true"),
                ("fastq.read.count", "gte", "1"),
                ("fastq.read.count", "lte", "5"),
                ("fastq.read.length.min", "gte", "8"),
                ("fastq.read.length.max", "lte", "100"),
                ("fastq.quality.encoding", "eq", "phred33"),
            ]
        );
    }

    #[test]
    fn invalid_file_suggests_only_the_validity_check() {
        let suggestions = FastqProvider
            .suggest(Path::new("tests/data/truncated.fastq"))
            .unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].metric, "fastq.valid");
    }

    #[test]
    fn ambiguous_encoding_is_not_suggested() {
        let suggestions = FastqProvider
            .suggest(Path::new("tests/data/ambiguous.fastq"))
            .unwrap();
        assert!(
            suggestions
                .iter()
                .all(|s| s.metric != "fastq.quality.encoding")
        );
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
