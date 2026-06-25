use crate::bam::functions::{read_header, reference_count};
use crate::core::BioAssertError;
use crate::suggest::Suggestion;
use crate::suggest::provider::SuggestionProvider;
use crate::suggest::providers::extension;
use std::path::Path;

/// Suggests the default checks for a BAM file: the reference-sequence (`@SQ`) count pinned with
/// `eq` (the reference set is a schema), and a read-group presence floor (`gte 1`).
pub struct BamProvider;

impl SuggestionProvider for BamProvider {
    fn name(&self) -> &'static str {
        "bam"
    }

    fn handles(&self, path: &Path) -> bool {
        extension(path).as_deref() == Some("bam")
    }

    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError> {
        let resource = path.to_string_lossy();
        let header = read_header(path)?;
        let references = reference_count(&header);

        Ok(vec![
            Suggestion::new(
                resource.as_ref(),
                "bam.header.sq.count",
                "eq",
                references.to_string(),
                Some(&format!(
                    "{references} reference sequence{}",
                    if references == 1 { "" } else { "s" }
                )),
            ),
            Suggestion::new(
                resource.as_ref(),
                "bam.header.rg.count",
                "gte",
                "1",
                Some("at least one read group"),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_bam_only() {
        assert!(BamProvider.handles(Path::new("a.bam")));
        assert!(BamProvider.handles(Path::new("A.BAM")));
        assert!(!BamProvider.handles(Path::new("a.fasta")));
    }

    #[test]
    fn sample_bam_one_reference_at_least_one_read_group() {
        let suggestions = BamProvider
            .suggest(Path::new("tests/data/sample.bam"))
            .unwrap();
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].metric, "bam.header.sq.count");
        assert_eq!(suggestions[0].comparator, "eq");
        assert_eq!(suggestions[0].expected, "1");
        assert_eq!(suggestions[1].metric, "bam.header.rg.count");
        assert_eq!(suggestions[1].comparator, "gte");
        assert_eq!(suggestions[1].expected, "1");
    }

    #[test]
    fn non_bam_input_errors() {
        // example.tsv is not a BAM, so reading the header fails.
        assert!(
            BamProvider
                .suggest(Path::new("tests/data/example.tsv"))
                .is_err()
        );
    }
}
