pub(crate) fn delimiter_for_prefix(prefix: &str) -> Option<char> {
    match prefix {
        "csv" => Some(','),
        "tsv" => Some('\t'),
        "psv" => Some('|'),
        _ => None,
    }
}

/// Maps a (lowercased) file extension to its metric prefix, the reverse direction of
/// [`delimiter_for_prefix`]. Extensions happen to equal prefixes today, but this is the
/// seam for future aliases (e.g. `.tab` mapping to `tsv`). Used by the `suggest` command
/// to pick the delimited family for a file by its extension.
pub(crate) fn prefix_for_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "csv" => Some("csv"),
        "tsv" => Some("tsv"),
        "psv" => Some("psv"),
        _ => None,
    }
}

pub(crate) fn parse_fields(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in line.chars() {
        if in_quotes {
            if ch == quote_char {
                in_quotes = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
        } else if ch == delimiter {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fields_splits_on_delimiter() {
        assert_eq!(parse_fields("a,b,c", ','), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_fields_handles_tab_delimiter() {
        assert_eq!(parse_fields("a\tb\tc", '\t'), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_fields_handles_pipe_delimiter() {
        assert_eq!(parse_fields("a|b|c", '|'), vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_fields_strips_double_quotes() {
        assert_eq!(
            parse_fields("\"hello world\",b", ','),
            vec!["hello world", "b"]
        );
    }

    #[test]
    fn parse_fields_strips_single_quotes() {
        assert_eq!(
            parse_fields("'hello world',b", ','),
            vec!["hello world", "b"]
        );
    }

    #[test]
    fn delimiter_for_prefix_maps_csv() {
        assert_eq!(delimiter_for_prefix("csv"), Some(','));
    }

    #[test]
    fn delimiter_for_prefix_maps_tsv() {
        assert_eq!(delimiter_for_prefix("tsv"), Some('\t'));
    }

    #[test]
    fn delimiter_for_prefix_maps_psv() {
        assert_eq!(delimiter_for_prefix("psv"), Some('|'));
    }

    #[test]
    fn delimiter_for_prefix_rejects_unknown() {
        assert_eq!(delimiter_for_prefix("dsv"), None);
    }

    #[test]
    fn prefix_for_extension_maps_known_extensions() {
        assert_eq!(prefix_for_extension("csv"), Some("csv"));
        assert_eq!(prefix_for_extension("tsv"), Some("tsv"));
        assert_eq!(prefix_for_extension("psv"), Some("psv"));
    }

    #[test]
    fn prefix_for_extension_rejects_unknown() {
        assert_eq!(prefix_for_extension("bam"), None);
        assert_eq!(prefix_for_extension("txt"), None);
    }
}
