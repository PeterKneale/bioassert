use bioassert_core::Assertion;
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
