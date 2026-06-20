use crate::assertion::Assertion;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "cli.pest"]
struct AssertionParser;

pub fn parse_assertion(input: &str) -> Result<Assertion, Box<dyn std::error::Error>> {
    let mut pairs = AssertionParser::parse(Rule::assertion, input)?;
    let mut inner = pairs.next().unwrap().into_inner();
    Ok(Assertion {
        file: inner.next().unwrap().as_str().to_string(),
        metric: inner.next().unwrap().as_str().to_string(),
        comparator: inner.next().unwrap().as_str().to_string(),
        expected: inner.next().unwrap().as_str().to_string(),
    })
}

pub fn parse_file(contents: &str) -> Result<Vec<Assertion>, Box<dyn std::error::Error>> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_assertion)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_assertion() {
        let a = parse_assertion("output.bam file.exists eq true").unwrap();
        assert_eq!(a.file, "output.bam");
        assert_eq!(a.metric, "file.exists");
        assert_eq!(a.comparator, "eq");
        assert_eq!(a.expected, "true");
    }

    // The grammar must consume the whole line; a value that merely starts with a
    // valid token ("true" in "truexx") must not be silently truncated to "true".
    #[test]
    fn rejects_value_with_trailing_characters() {
        assert!(parse_assertion("output.bam file.exists eq truexx").is_err());
    }

    #[test]
    fn rejects_an_extra_trailing_token() {
        assert!(parse_assertion("output.bam file.exists eq true xx").is_err());
    }
}
