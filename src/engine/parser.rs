use crate::engine::assertion::{Assertion, Condition, Guard};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "engine/assertions.pest"]
struct AssertionParser;

pub fn parse_assertion(input: &str) -> Result<Assertion, Box<dyn std::error::Error>> {
    let mut pairs = AssertionParser::parse(Rule::assertion, input)?;
    let mut inner = pairs.next().unwrap().into_inner();
    let file = inner.next().unwrap().as_str().to_string();
    let metric = inner.next().unwrap().as_str().to_string();
    let comparator = inner.next().unwrap().as_str().to_string();
    let expected = inner.next().unwrap().as_str().to_string();

    // An optional guard appears as a `guard_keyword` pair followed by a `condition` pair.
    // The trailing pair is otherwise `EOI`, which is ignored.
    let guard = match inner.next() {
        Some(keyword) if keyword.as_rule() == Rule::guard_keyword => {
            let negate = keyword.as_str().eq_ignore_ascii_case("unless");
            let condition = parse_condition(inner.next().unwrap(), &file);
            Some(Guard { negate, condition })
        }
        _ => None,
    };

    Ok(Assertion { file, metric, comparator, expected, guard })
}

/// Builds a [`Condition`] from a `condition` pair. The full form carries its own file,
/// metric, comparator and value; the shorthand form is a bare metric and is expanded to
/// a condition on the assertion's own `file` with `eq true`.
fn parse_condition(condition: pest::iterators::Pair<Rule>, assertion_file: &str) -> Condition {
    let inner = condition.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::full_condition => {
            let mut parts = inner.into_inner();
            Condition {
                file: parts.next().unwrap().as_str().to_string(),
                metric: parts.next().unwrap().as_str().to_string(),
                comparator: parts.next().unwrap().as_str().to_string(),
                expected: parts.next().unwrap().as_str().to_string(),
            }
        }
        Rule::metric_condition => Condition {
            file: assertion_file.to_string(),
            metric: inner.as_str().to_string(),
            comparator: "eq".to_string(),
            expected: "true".to_string(),
        },
        rule => unreachable!("condition is full_condition or metric_condition, got {rule:?}"),
    }
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
        assert!(a.guard.is_none());
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

    #[test]
    fn parses_shorthand_guard_against_the_assertion_file() {
        let a = parse_assertion("data.tsv tsv.columns.count eq 18 if file.exists").unwrap();
        assert_eq!(a.metric, "tsv.columns.count");
        let guard = a.guard.expect("expected a guard");
        assert!(!guard.negate);
        // shorthand expands to the assertion's own file with `eq true`
        assert_eq!(guard.condition.file, "data.tsv");
        assert_eq!(guard.condition.metric, "file.exists");
        assert_eq!(guard.condition.comparator, "eq");
        assert_eq!(guard.condition.expected, "true");
    }

    #[test]
    fn parses_unless_as_a_negated_guard() {
        let a = parse_assertion("data.tsv tsv.columns.count eq 18 unless file.empty").unwrap();
        let guard = a.guard.expect("expected a guard");
        assert!(guard.negate);
        assert_eq!(guard.condition.metric, "file.empty");
    }

    #[test]
    fn parses_full_form_guard_against_another_file() {
        let a = parse_assertion("out.tsv tsv.line.count gt 0 if other.bam bam.header.rg.count gt 0").unwrap();
        let guard = a.guard.expect("expected a guard");
        assert!(!guard.negate);
        assert_eq!(guard.condition.file, "other.bam");
        assert_eq!(guard.condition.metric, "bam.header.rg.count");
        assert_eq!(guard.condition.comparator, "gt");
        assert_eq!(guard.condition.expected, "0");
    }

    #[test]
    fn guard_keyword_is_case_insensitive() {
        assert!(parse_assertion("data.tsv tsv.columns.count eq 18 IF file.exists").is_ok());
        assert!(parse_assertion("data.tsv tsv.columns.count eq 18 UNLESS file.empty").is_ok());
    }

    #[test]
    fn rejects_guard_keyword_without_a_condition() {
        assert!(parse_assertion("data.tsv tsv.columns.count eq 18 if").is_err());
    }
}
