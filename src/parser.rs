use pest::Parser;
use pest_derive::Parser;

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


pub fn parse_raw_assertion(input: &str) -> Result<Assertion, Box<dyn std::error::Error>> {
    // Note that we are parsing a single assertion here by specifying the rule
    let mut pairs = AssertionParser::parse(Rule::assertion, input)?;

    let assertion = pairs.next().unwrap();

    let mut inner = assertion.into_inner();

    let model = Assertion {
        file: inner.next().unwrap().as_str().to_string(),
        metric: inner.next().unwrap().as_str().to_string(),
        comparator: inner.next().unwrap().as_str().to_string(),
        expected: inner.next().unwrap().as_str().to_string(),
    };
    Ok(model)
}

pub fn parse_file(contents: &str) -> Result<Vec<Assertion>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }
        if line.starts_with("#"){
            continue;
        }

        result.push(parse_raw_assertion(line)?);
    }

    Ok(result)
}
