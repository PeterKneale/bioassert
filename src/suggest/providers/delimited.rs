use crate::core::{BioAssertError, Value};
use crate::delimited::column_count::functions::column_count;
use crate::delimited::functions::{delimiter_for_prefix, prefix_for_extension};
use crate::delimited::line_count::functions::line_count;
use crate::suggest::Suggestion;
use crate::suggest::provider::SuggestionProvider;
use crate::suggest::providers::extension;
use crate::suggest::suggestion::band;
use std::path::Path;

/// Suggests the default checks for a delimited file (`.tsv`, `.csv`, `.psv`): the column count
/// is pinned with `eq` (a change there is a schema regression), while the row count is given a
/// +/- 50% band (rows legitimately vary run to run).
pub struct DelimitedProvider;

impl SuggestionProvider for DelimitedProvider {
    fn name(&self) -> &'static str {
        "delimited"
    }

    fn handles(&self, path: &Path) -> bool {
        extension(path)
            .and_then(|ext| prefix_for_extension(&ext))
            .is_some()
    }

    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError> {
        let resource = path.to_string_lossy();
        // `handles` guarantees an extension that maps to a prefix and a delimiter.
        let prefix = extension(path)
            .and_then(|ext| prefix_for_extension(&ext))
            .expect("DelimitedProvider::suggest called for an unsupported extension");
        let delimiter =
            delimiter_for_prefix(prefix).expect("a delimited prefix always maps to a delimiter");

        let columns = column_count(path, delimiter)?;
        let rows = integer_of(&line_count(path)?);
        let (lower, upper) = band(rows);

        Ok(vec![
            Suggestion::new(
                resource.as_ref(),
                format!("{prefix}.columns.count"),
                "eq",
                columns.to_string(),
                Some(&format!("expected {columns} columns")),
            ),
            Suggestion::new(
                resource.as_ref(),
                format!("{prefix}.lines.count"),
                "gte",
                lower.to_string(),
                Some(&format!("rows within +/- 50% of {rows}")),
            ),
            Suggestion::new(
                resource.as_ref(),
                format!("{prefix}.lines.count"),
                "lte",
                upper.to_string(),
                None,
            ),
        ])
    }
}

/// Extracts the count from an integer-valued property. The delimited property functions return
/// `IntegerValue`; any other variant is not expected here and yields 0.
fn integer_of(value: &Value) -> u64 {
    match value {
        Value::IntegerValue(n) | Value::BytesValue(n) => *n,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_delimited_extensions_only() {
        assert!(DelimitedProvider.handles(Path::new("a.tsv")));
        assert!(DelimitedProvider.handles(Path::new("a.csv")));
        assert!(DelimitedProvider.handles(Path::new("a.psv")));
        assert!(DelimitedProvider.handles(Path::new("A.TSV")));
        assert!(!DelimitedProvider.handles(Path::new("a.bam")));
        assert!(!DelimitedProvider.handles(Path::new("a")));
    }

    #[test]
    fn example_tsv_three_columns_three_lines() {
        let suggestions = DelimitedProvider
            .suggest(Path::new("tests/data/example.tsv"))
            .unwrap();
        assert_eq!(suggestions.len(), 3);
        assert_eq!(suggestions[0].metric, "tsv.columns.count");
        assert_eq!(suggestions[0].comparator, "eq");
        assert_eq!(suggestions[0].expected, "3");
        assert_eq!(suggestions[1].metric, "tsv.lines.count");
        assert_eq!(suggestions[1].comparator, "gte");
        assert_eq!(suggestions[1].expected, "1");
        assert_eq!(suggestions[2].comparator, "lte");
        assert_eq!(suggestions[2].expected, "5");
    }

    #[test]
    fn junctions_tsv_twelve_columns_four_lines() {
        let suggestions = DelimitedProvider
            .suggest(Path::new("tests/data/junctions.tsv"))
            .unwrap();
        assert_eq!(suggestions[0].expected, "12");
        assert_eq!(suggestions[1].expected, "2");
        assert_eq!(suggestions[2].expected, "6");
    }
}
