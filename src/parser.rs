use pest::Parser;
use pest_derive::Parser;
use std::str::FromStr;

#[derive(Parser)]
#[grammar = "cli.pest"]
pub struct AssertionParser;

#[derive(Debug)]
pub struct Assertion {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
}

impl FromStr for Assertion {
    type Err = Box<dyn std::error::Error>;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut pairs = AssertionParser::parse(Rule::assertion, input)?;
        let mut inner = pairs.next().unwrap().into_inner();
        Ok(Self {
            file: inner.next().unwrap().as_str().to_string(),
            metric: inner.next().unwrap().as_str().to_string(),
            comparator: inner.next().unwrap().as_str().to_string(),
            expected: inner.next().unwrap().as_str().to_string(),
        })
    }
}

pub fn parse_file(contents: &str) -> Result<Vec<Assertion>, Box<dyn std::error::Error>> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::parse)
        .collect()
}
