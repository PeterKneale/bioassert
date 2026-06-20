pub(crate) fn delimiter_for_prefix(prefix: &str) -> Option<char> {
    match prefix {
        "csv" => Some(','),
        "tsv" => Some('\t'),
        "psv" => Some('|'),
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
        assert_eq!(parse_fields("\"hello world\",b", ','), vec!["hello world", "b"]);
    }

    #[test]
    fn parse_fields_strips_single_quotes() {
        assert_eq!(parse_fields("'hello world',b", ','), vec!["hello world", "b"]);
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
}
