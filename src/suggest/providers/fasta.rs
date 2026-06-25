use crate::core::BioAssertError;
use crate::fasta::functions::{read_records, record_count, total_length};
use crate::suggest::Suggestion;
use crate::suggest::provider::SuggestionProvider;
use crate::suggest::providers::extension;
use crate::suggest::suggestion::band;
use std::path::Path;

/// Suggests the default checks for a FASTA file (`.fasta`, `.fa`, `.fna`): the record count
/// pinned with `eq` (the record set is a schema), and a +/- 50% band on the total sequence
/// length (which varies run to run).
pub struct FastaProvider;

impl SuggestionProvider for FastaProvider {
    fn name(&self) -> &'static str {
        "fasta"
    }

    fn handles(&self, path: &Path) -> bool {
        matches!(extension(path).as_deref(), Some("fasta" | "fa" | "fna"))
    }

    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError> {
        let resource = path.to_string_lossy();
        let records = read_records(path)?;
        let count = record_count(&records);
        let length = total_length(&records);
        let (lower, upper) = band(length);

        Ok(vec![
            Suggestion::new(
                resource.as_ref(),
                "fasta.seq.count",
                "eq",
                count.to_string(),
                Some(&format!(
                    "{count} sequence record{}",
                    if count == 1 { "" } else { "s" }
                )),
            ),
            Suggestion::new(
                resource.as_ref(),
                "fasta.length",
                "gte",
                lower.to_string(),
                Some(&format!("total length within +/- 50% of {length}")),
            ),
            Suggestion::new(
                resource.as_ref(),
                "fasta.length",
                "lte",
                upper.to_string(),
                None,
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_fasta_extensions() {
        assert!(FastaProvider.handles(Path::new("a.fasta")));
        assert!(FastaProvider.handles(Path::new("a.fa")));
        assert!(FastaProvider.handles(Path::new("a.fna")));
        assert!(FastaProvider.handles(Path::new("A.FASTA")));
        assert!(!FastaProvider.handles(Path::new("a.tsv")));
    }

    #[test]
    fn sample_fasta_three_records_length_42() {
        let suggestions = FastaProvider
            .suggest(Path::new("tests/data/sample.fasta"))
            .unwrap();
        assert_eq!(suggestions.len(), 3);
        assert_eq!(suggestions[0].metric, "fasta.seq.count");
        assert_eq!(suggestions[0].expected, "3");
        assert_eq!(suggestions[1].metric, "fasta.length");
        assert_eq!(suggestions[1].comparator, "gte");
        assert_eq!(suggestions[1].expected, "21");
        assert_eq!(suggestions[2].comparator, "lte");
        assert_eq!(suggestions[2].expected, "63");
    }

    #[test]
    fn empty_fasta_zero_records_degenerate_band() {
        let suggestions = FastaProvider
            .suggest(Path::new("tests/data/empty.fasta"))
            .unwrap();
        assert_eq!(suggestions[0].expected, "0");
        assert_eq!(suggestions[1].expected, "0");
        assert_eq!(suggestions[2].expected, "0");
    }
}
