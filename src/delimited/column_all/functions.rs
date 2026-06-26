use crate::core::{FileError, StringMatcher};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

/// The outcome of applying a matcher to every value of a delimited column.
pub enum ColumnCheck {
    /// Every checked row satisfied the matcher; carries how many rows were checked.
    AllMatch { checked: usize },
    /// A row failed; carries its 1-indexed line number and the offending cell value.
    Mismatch { line: usize, value: String },
}

/// Streams `column` (1-indexed) across the file's rows, applying `matcher` to each cell
/// and stopping at the first mismatch (so a huge file is not read past the first failing
/// row). When `skip_header` is true the first line is not checked. A header-only or empty
/// file yields `AllMatch { checked: 0 }`, a vacuous pass. A row with fewer than `column`
/// fields is a structural error.
pub fn check_column(
    file: &Path,
    delimiter: char,
    column: usize,
    skip_header: bool,
    matcher: &StringMatcher,
) -> Result<ColumnCheck, FileError> {
    let reader = io::BufReader::new(File::open(file).map_err(|e| FileError::new(file, e))?);
    let mut checked = 0usize;
    for (idx, line) in reader.lines().enumerate() {
        let line_no = idx + 1;
        if skip_header && line_no == 1 {
            continue;
        }
        let raw = line.map_err(|e| FileError::new(file, e))?;
        let value = super::super::functions::parse_fields(&raw, delimiter)
            .into_iter()
            .nth(column - 1)
            .ok_or_else(|| {
                FileError::new(
                    file,
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("column {} not found on line {}", column, line_no),
                    ),
                )
            })?;
        checked += 1;
        if !matcher.is_match(&value) {
            return Ok(ColumnCheck::Mismatch {
                line: line_no,
                value,
            });
        }
    }
    Ok(ColumnCheck::AllMatch { checked })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Comparator, Operator};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_file(contents: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f
    }

    fn matcher(op: Operator, expected: &str) -> StringMatcher {
        Comparator::new(op).string_matcher(expected).unwrap()
    }

    #[test]
    fn all_rows_match_includes_header() {
        let f = temp_file("strand\n+\n-\n");
        let m = matcher(Operator::Matches, "^[+-]$");
        // line 1 ("strand") is checked and does not match
        assert!(matches!(
            check_column(f.path(), '\t', 1, false, &m).unwrap(),
            ColumnCheck::Mismatch { line: 1, .. }
        ));
    }

    #[test]
    fn skip_header_checks_only_data_rows() {
        let f = temp_file("strand\n+\n-\n+\n");
        let m = matcher(Operator::Matches, "^[+-]$");
        assert!(matches!(
            check_column(f.path(), '\t', 1, true, &m).unwrap(),
            ColumnCheck::AllMatch { checked: 3 }
        ));
    }

    #[test]
    fn reports_first_failing_row_and_value() {
        let f = temp_file("a\tb\nJUNC1\tx\nNOPE\ty\n");
        let m = matcher(Operator::Matches, "^JUNC[0-9]+$");
        match check_column(f.path(), '\t', 1, true, &m).unwrap() {
            ColumnCheck::Mismatch { line, value } => {
                assert_eq!(line, 3);
                assert_eq!(value, "NOPE");
            }
            ColumnCheck::AllMatch { .. } => panic!("expected a mismatch"),
        }
    }

    #[test]
    fn header_only_file_passes_vacuously_when_skipping_header() {
        let f = temp_file("strand\n");
        let m = matcher(Operator::Matches, "^[+-]$");
        assert!(matches!(
            check_column(f.path(), '\t', 1, true, &m).unwrap(),
            ColumnCheck::AllMatch { checked: 0 }
        ));
    }

    #[test]
    fn empty_file_passes_vacuously() {
        let f = temp_file("");
        let m = matcher(Operator::Matches, "^[+-]$");
        assert!(matches!(
            check_column(f.path(), '\t', 1, false, &m).unwrap(),
            ColumnCheck::AllMatch { checked: 0 }
        ));
    }

    #[test]
    fn missing_column_on_a_row_is_an_error() {
        let f = temp_file("a\tb\tc\nx\ty\tz\nshort\n");
        let m = matcher(Operator::Matches, ".*");
        assert!(check_column(f.path(), '\t', 3, true, &m).is_err());
    }

    #[test]
    fn supports_non_regex_comparators() {
        let f = temp_file("n\n10\n20\n30\n");
        let m = matcher(Operator::Ne, "0");
        assert!(matches!(
            check_column(f.path(), '\t', 1, true, &m).unwrap(),
            ColumnCheck::AllMatch { checked: 3 }
        ));
    }
}
