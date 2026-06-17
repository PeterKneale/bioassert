mod delimited_cell;
mod delimited_column_count;
mod delimited_line_count;
mod file_empty;
mod file_exists;
mod file_lines;
mod file_size;

pub use delimited_cell::DelimitedCellExecutor;
pub use delimited_column_count::DelimitedColumnCountExecutor;
pub use delimited_line_count::DelimitedLineCountExecutor;
pub use file_empty::FileEmptyExecutor;
pub use file_exists::FileExistsExecutor;
pub use file_lines::FileLinesExecutor;
pub use file_size::FileSizeExecutor;

use crate::parser::Assertion;

pub trait MetricExecutor {
    fn execute(self, assertion: Assertion) -> Result<(bool, String), Box<dyn std::error::Error>>;
}

fn parse_fields(line: &str, delimiter: char) -> Vec<String> {
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
}
