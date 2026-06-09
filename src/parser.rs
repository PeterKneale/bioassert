//! Assertion file parsing: plain-text DSL and YAML.
//!
//! Spec: `docs/spec.md` → "Assertion File Formats and Examples" and the locked DSL grammar
//! (`<subject> <metric> <operator> <value>`, explicit booleans, relational/cross-subject RHS,
//! SI decimal size literals).
//!
//! The parser is format-agnostic at the `run_assertions` boundary: [`parse`] sniffs whether the
//! source is a YAML bundle or the line-oriented DSL and dispatches accordingly.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::model::{Assertion, Operator, Value};

/// An error encountered while parsing an assertion source.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// A plain-text DSL error, with the 1-based line number.
    #[error("line {line}: {message}")]
    Dsl { line: usize, message: String },
    /// A YAML deserialization or expression error.
    #[error("YAML error: {0}")]
    Yaml(String),
}

/// The result of parsing one assertion source.
///
/// `inputs` carries any bindings declared in a YAML bundle; CLI `--input` bindings are merged on
/// top of these by the caller (`run_assertions`).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ParsedBundle {
    pub name: Option<String>,
    pub inputs: HashMap<String, PathBuf>,
    pub assertions: Vec<Assertion>,
}

/// Parse an assertion source (auto-detecting YAML vs. the plain-text DSL).
pub fn parse(source: &str) -> Result<ParsedBundle, ParseError> {
    if looks_like_yaml(source) {
        parse_yaml(source)
    } else {
        parse_dsl(source)
    }
}

/// Heuristic: the first meaningful (non-blank, non-comment) line of a YAML bundle starts with one
/// of the known top-level keys. DSL lines never start with `key:` tokens.
fn looks_like_yaml(source: &str) -> bool {
    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        return line.starts_with("name:")
            || line.starts_with("inputs:")
            || line.starts_with("assertions:");
    }
    false
}

// ---------------------------------------------------------------------------
// Plain-text DSL
// ---------------------------------------------------------------------------

fn parse_dsl(source: &str) -> Result<ParsedBundle, ParseError> {
    let mut assertions = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        let assertion = parse_expression(line).map_err(|message| ParseError::Dsl {
            line: line_no,
            message,
        })?;
        assertions.push(assertion);
    }
    Ok(ParsedBundle {
        name: None,
        inputs: HashMap::new(),
        assertions,
    })
}

/// Parse a single assertion expression `<subject> <metric> <operator> <value>`.
///
/// The value is the remainder of the line (so it may contain spaces, e.g. quoted strings and
/// bracketed lists). Boolean shorthand (`<subject> <metric>`) is rejected.
fn parse_expression(text: &str) -> Result<Assertion, String> {
    let (subject, rest) = take_token(text).ok_or_else(|| "empty assertion".to_string())?;
    let (metric, rest) = take_token(rest)
        .ok_or_else(|| "expected `<subject> <metric> <operator> <value>`".to_string())?;
    let (op_token, rest) = take_token(rest).ok_or_else(|| {
        format!("boolean metrics must be explicit, e.g. `{subject} {metric} eq true`")
    })?;

    let value_str = rest.trim();
    if value_str.is_empty() {
        return Err(format!(
            "missing value; boolean metrics must be explicit, e.g. `{subject} {metric} eq true`"
        ));
    }

    let op =
        Operator::from_dsl(op_token).ok_or_else(|| format!("unknown operator `{op_token}`"))?;
    let expected = Some(parse_value(value_str)?);

    Ok(Assertion {
        subject: subject.to_string(),
        metric: metric.to_string(),
        op,
        expected,
    })
}

/// Split off the first whitespace-delimited token, returning `(token, remainder)`.
fn take_token(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    match s.find(char::is_whitespace) {
        Some(idx) => Some((&s[..idx], &s[idx..])),
        None => Some((s, "")),
    }
}

/// Parse a value literal. Bare words (including bound-subject references) become `String`; the
/// engine resolves whether a `String` is a literal or a cross-subject reference.
fn parse_value(s: &str) -> Result<Value, String> {
    let s = s.trim();

    // List: [a, b, c]
    if let Some(inner) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        let mut items = Vec::new();
        for part in inner.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            items.push(parse_value(part)?);
        }
        return Ok(Value::List(items));
    }

    // Quoted string: "..."
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Ok(Value::String(s[1..s.len() - 1].to_string()));
    }

    // Boolean
    match s {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        _ => {}
    }

    // SI decimal size literal: 100MB, 1.5GB, ...
    if let Some(bytes) = parse_size(s) {
        return Ok(Value::Integer(bytes));
    }

    // Integer / float
    if let Ok(i) = s.parse::<u64>() {
        return Ok(Value::Integer(i));
    }
    if let Ok(x) = s.parse::<f64>() {
        return Ok(Value::Float(x));
    }

    // Bare word string (covers plain strings, regex patterns, and subject references)
    Ok(Value::String(s.to_string()))
}

/// Parse an SI decimal size literal (`KB`, `MB`, `GB`, `TB`) into a byte count.
/// Returns `None` when `s` is not a size literal (so the caller can fall through to other types).
fn parse_size(s: &str) -> Option<u64> {
    const UNITS: &[(&str, u64)] = &[
        ("KB", 1_000),
        ("MB", 1_000_000),
        ("GB", 1_000_000_000),
        ("TB", 1_000_000_000_000),
    ];
    for &(suffix, mult) in UNITS {
        if let Some(num) = s.strip_suffix(suffix) {
            let num = num.trim();
            if num.is_empty() {
                return None;
            }
            if let Ok(f) = num.parse::<f64>()
                && f.is_finite()
                && f >= 0.0
            {
                return Some((f * mult as f64).round() as u64);
            }
            return None;
        }
    }
    None
}

/// Remove a trailing `#` comment, respecting double-quoted strings and `[ ]` brackets.
/// A leading `#` (full-line comment) yields an empty string.
fn strip_comment(line: &str) -> &str {
    let mut in_quote = false;
    let mut depth: i32 = 0;
    let mut prev_ws = true;
    for (i, b) in line.bytes().enumerate() {
        match b {
            b'"' => in_quote = !in_quote,
            b'[' if !in_quote => depth += 1,
            b']' if !in_quote => depth -= 1,
            b'#' if !in_quote && depth <= 0 && prev_ws => return &line[..i],
            _ => {}
        }
        prev_ws = (b as char).is_whitespace();
    }
    line
}

// ---------------------------------------------------------------------------
// YAML
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBundle {
    name: Option<String>,
    #[serde(default)]
    inputs: HashMap<String, PathBuf>,
    #[serde(default)]
    assertions: Vec<RawAssertion>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum RawAssertion {
    /// Shorthand: a single expression string.
    Expr(String),
    /// Explicit form with an optional name and per-assertion input bindings.
    Explicit(ExplicitAssertion),
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplicitAssertion {
    /// Human-readable label (metadata; not yet surfaced in reports).
    #[allow(dead_code)]
    name: Option<String>,
    expression: String,
    #[serde(default)]
    inputs: HashMap<String, PathBuf>,
}

fn parse_yaml(source: &str) -> Result<ParsedBundle, ParseError> {
    let raw: RawBundle =
        serde_yaml::from_str(source).map_err(|e| ParseError::Yaml(e.to_string()))?;

    let mut inputs = raw.inputs;
    let mut assertions = Vec::new();

    for item in raw.assertions {
        let (expression, extra_inputs) = match item {
            RawAssertion::Expr(expr) => (expr, None),
            RawAssertion::Explicit(explicit) => (explicit.expression, Some(explicit.inputs)),
        };
        if let Some(extra) = extra_inputs {
            inputs.extend(extra);
        }
        let assertion = parse_expression(expression.trim()).map_err(|message| {
            ParseError::Yaml(format!("invalid expression `{expression}`: {message}"))
        })?;
        assertions.push(assertion);
    }

    Ok(ParsedBundle {
        name: raw.name,
        inputs,
        assertions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dsl(line: &str) -> Assertion {
        let bundle = parse(line).expect("parses");
        assert_eq!(bundle.assertions.len(), 1, "expected exactly one assertion");
        bundle.assertions.into_iter().next().unwrap()
    }

    #[test]
    fn parses_basic_numeric_assertion() {
        let a = dsl("bam read_count gt 1000000");
        assert_eq!(a.subject, "bam");
        assert_eq!(a.metric, "read_count");
        assert_eq!(a.op, Operator::Gt);
        assert_eq!(a.expected, Some(Value::Integer(1_000_000)));
    }

    #[test]
    fn maps_dsl_operators_to_enum() {
        assert_eq!(dsl("bam read_count gte 1").op, Operator::Ge);
        assert_eq!(dsl("bam read_count lte 1").op, Operator::Le);
        assert_eq!(dsl("bam contigs not_in [chrM]").op, Operator::NotIn);
    }

    #[test]
    fn parses_boolean_explicitly() {
        let a = dsl("bam has_index eq true");
        assert_eq!(a.metric, "has_index");
        assert_eq!(a.op, Operator::Eq);
        assert_eq!(a.expected, Some(Value::Bool(true)));
    }

    #[test]
    fn parses_si_size_literals() {
        assert_eq!(
            dsl("bam size gte 100MB").expected,
            Some(Value::Integer(100_000_000))
        );
        assert_eq!(
            dsl("bam size gte 1KB").expected,
            Some(Value::Integer(1_000))
        );
        assert_eq!(
            dsl("bam size gte 1.5GB").expected,
            Some(Value::Integer(1_500_000_000))
        );
        assert_eq!(
            dsl("bam size gte 2TB").expected,
            Some(Value::Integer(2_000_000_000_000))
        );
    }

    #[test]
    fn parses_list_value() {
        let a = dsl("vcf contigs contains [chr1, chr2]");
        assert_eq!(
            a.expected,
            Some(Value::List(vec![
                Value::String("chr1".into()),
                Value::String("chr2".into()),
            ]))
        );
    }

    #[test]
    fn parses_quoted_string_with_spaces() {
        let a = dsl("fq quality_encoding eq \"illumina phred+33\"");
        assert_eq!(a.expected, Some(Value::String("illumina phred+33".into())));
    }

    #[test]
    fn parses_relational_and_cross_subject_as_bare_subject() {
        let rel = dsl("read1 paired_with eq read2");
        assert_eq!(rel.metric, "paired_with");
        assert_eq!(rel.expected, Some(Value::String("read2".into())));

        let cross = dsl("read1 read_count eq read2");
        assert_eq!(cross.expected, Some(Value::String("read2".into())));
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let src = "\n# a full line comment\nbam read_count gt 10  # trailing comment\n\n";
        let bundle = parse(src).expect("parses");
        assert_eq!(bundle.assertions.len(), 1);
        assert_eq!(bundle.assertions[0].expected, Some(Value::Integer(10)));
    }

    #[test]
    fn hash_inside_quotes_is_not_a_comment() {
        let a = dsl("fq quality_encoding eq \"phred#33\"");
        assert_eq!(a.expected, Some(Value::String("phred#33".into())));
    }

    #[test]
    fn rejects_boolean_shorthand() {
        let err = parse("bam exists").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("boolean metrics must be explicit"),
            "got: {msg}"
        );
        assert!(msg.contains("bam exists eq true"), "got: {msg}");
    }

    #[test]
    fn rejects_missing_value() {
        let err = parse("bam read_count gt").unwrap_err();
        assert!(err.to_string().contains("missing value"), "got: {err}");
    }

    #[test]
    fn rejects_unknown_operator() {
        let err = parse("bam read_count >> 10").unwrap_err();
        assert!(err.to_string().contains("unknown operator"), "got: {err}");
    }

    #[test]
    fn reports_line_number_on_error() {
        let src = "bam read_count gt 10\nbam exists\n";
        match parse(src).unwrap_err() {
            ParseError::Dsl { line, .. } => assert_eq!(line, 2),
            other => panic!("expected DSL error, got {other:?}"),
        }
    }

    #[test]
    fn parses_yaml_bundle() {
        let src = r#"
name: aligned_bam_checks
inputs:
  bam: sample.bam
assertions:
  - bam read_count gt 100000
  - bam sort_order eq coordinate
  - bam has_index eq true
"#;
        let bundle = parse(src).expect("parses yaml");
        assert_eq!(bundle.name.as_deref(), Some("aligned_bam_checks"));
        assert_eq!(bundle.inputs.get("bam"), Some(&PathBuf::from("sample.bam")));
        assert_eq!(bundle.assertions.len(), 3);
        assert_eq!(bundle.assertions[2].metric, "has_index");
        assert_eq!(bundle.assertions[2].expected, Some(Value::Bool(true)));
    }

    #[test]
    fn parses_yaml_paired_fastq_bundle() {
        let src = r#"
name: paired_fastqs
inputs:
  read1: reads_R1.fastq.gz
  read2: reads_R2.fastq.gz
assertions:
  - read1 paired_with eq read2
  - read1 read_count eq read2
"#;
        let bundle = parse(src).expect("parses yaml");
        assert_eq!(bundle.inputs.len(), 2);
        assert_eq!(bundle.assertions[0].metric, "paired_with");
        assert_eq!(
            bundle.assertions[1].expected,
            Some(Value::String("read2".into()))
        );
    }

    #[test]
    fn parses_yaml_explicit_style_with_scoped_inputs() {
        let src = r#"
assertions:
  - name: tumour_indexed
    expression: bam has_index eq true
    inputs:
      bam: tumour.bam
  - name: fastq_paired
    expression: read1 paired_with eq read2
    inputs:
      read1: sample_R1.fq.gz
      read2: sample_R2.fq.gz
"#;
        let bundle = parse(src).expect("parses yaml");
        assert_eq!(bundle.assertions.len(), 2);
        assert_eq!(bundle.inputs.get("bam"), Some(&PathBuf::from("tumour.bam")));
        assert_eq!(
            bundle.inputs.get("read1"),
            Some(&PathBuf::from("sample_R1.fq.gz"))
        );
        assert_eq!(bundle.inputs.len(), 3);
    }

    #[test]
    fn yaml_expression_error_is_reported_as_yaml() {
        let src = "assertions:\n  - bam exists\n";
        match parse(src).unwrap_err() {
            ParseError::Yaml(msg) => assert!(
                msg.contains("boolean metrics must be explicit"),
                "got: {msg}"
            ),
            other => panic!("expected YAML error, got {other:?}"),
        }
    }

    #[test]
    fn detects_dsl_vs_yaml() {
        assert!(!looks_like_yaml("bam exists eq true"));
        assert!(looks_like_yaml("# c\nassertions:\n  - bam exists eq true"));
        assert!(looks_like_yaml("name: x\n"));
    }
}
