//! Assertion evaluation engine.
//!
//! Spec: `docs/spec.md` → "Runner", "Rich comparators", "Assertion Evaluation Flow".
//!
//! The engine is decoupled from metric providers via the [`MetricResolver`] trait: the metric
//! registry (Phase 4) implements it for real files, while tests use a stub. The engine handles
//! subject resolution, comparator execution, cross-subject references, fail-fast vs. report-all,
//! and `Report` construction.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::model::{Assertion, AssertionResult, Operator, Report, Status, Value};

/// Computes metric values for subjects. Implemented by the metric registry (Phase 4); a stub is
/// used in tests.
pub trait MetricResolver {
    /// Compute `metric` for the file bound to `subject` at `path`.
    fn resolve(&mut self, subject: &str, path: &Path, metric: &str) -> Result<Value>;
}

/// Evaluate `assertions` against bound `inputs` using `resolver`.
///
/// * Unbound subjects and metric/comparison errors are hard errors (exit code 2).
/// * A `false` comparison records a `Fail`; when `continue_on_failure` is `false`, evaluation
///   stops after the first failure (fail-fast).
pub fn evaluate(
    assertions: &[Assertion],
    inputs: &HashMap<String, PathBuf>,
    resolver: &mut dyn MetricResolver,
    continue_on_failure: bool,
) -> Result<Report> {
    let mut report = Report::default();

    for assertion in assertions {
        let path = inputs.get(&assertion.subject).ok_or_else(|| {
            anyhow!(
                "input `{0}` is not bound; provide --input {0}=<path>",
                assertion.subject
            )
        })?;

        let actual = resolver
            .resolve(&assertion.subject, path, &assertion.metric)
            .with_context(|| format!("computing {}.{}", assertion.subject, assertion.metric))?;

        let expected = assertion.expected.as_ref().ok_or_else(|| {
            anyhow!(
                "assertion for {}.{} has no expected value",
                assertion.subject,
                assertion.metric
            )
        })?;

        // Resolve a bare-subject RHS (e.g. `read1 read_count eq read2`) by computing the *same*
        // metric on the referenced subject and comparing the two values.
        let resolved_expected = match expected {
            Value::String(name) if inputs.contains_key(name) => {
                let other_path = &inputs[name];
                resolver
                    .resolve(name, other_path, &assertion.metric)
                    .with_context(|| format!("computing {}.{}", name, assertion.metric))?
            }
            other => other.clone(),
        };

        let passed = compare(&actual, assertion.op, &resolved_expected)
            .with_context(|| format!("comparing {}.{}", assertion.subject, assertion.metric))?;

        let status = if passed { Status::Pass } else { Status::Fail };
        let message = format_message(
            &assertion.metric,
            assertion.op,
            &actual,
            &resolved_expected,
            status,
        );

        report.results.push(AssertionResult {
            subject: assertion.subject.clone(),
            resolved_path: Some(path.clone()),
            metric: assertion.metric.clone(),
            op: assertion.op,
            expected: Some(resolved_expected),
            actual: Some(actual),
            status,
            message,
        });

        if status == Status::Fail && !continue_on_failure {
            break;
        }
    }

    Ok(report)
}

/// Build the human-readable diagnostic stored on an [`AssertionResult`].
fn format_message(
    metric: &str,
    op: Operator,
    actual: &Value,
    expected: &Value,
    status: Status,
) -> String {
    match status {
        Status::Pass => format!("{metric}={actual} {op} {expected}"),
        Status::Fail => format!("{metric}={actual}, expected {op} {expected}"),
    }
}

/// Apply a comparison operator. Type mismatches (e.g. `gt` on a string) are hard errors.
pub fn compare(actual: &Value, op: Operator, expected: &Value) -> Result<bool> {
    match op {
        Operator::Eq => Ok(values_equal(actual, expected)),
        Operator::Ne => Ok(!values_equal(actual, expected)),
        Operator::Gt | Operator::Lt | Operator::Ge | Operator::Le => {
            let a = as_f64(actual).ok_or_else(|| {
                anyhow!("operator `{op}` requires a numeric value, got `{actual}`")
            })?;
            let b = as_f64(expected).ok_or_else(|| {
                anyhow!("operator `{op}` requires a numeric expected value, got `{expected}`")
            })?;
            Ok(match op {
                Operator::Gt => a > b,
                Operator::Lt => a < b,
                Operator::Ge => a >= b,
                Operator::Le => a <= b,
                _ => unreachable!(),
            })
        }
        Operator::In => membership(actual, expected),
        Operator::NotIn => Ok(!membership(actual, expected)?),
        Operator::Contains => contains(actual, expected),
        Operator::Matches => matches_regex(actual, expected),
    }
}

/// Value equality with numeric coercion between `Integer` and `Float`.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(l, r)| values_equal(l, r))
        }
        _ => match (as_f64(a), as_f64(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

/// Numeric view of a value, if it is an `Integer` or `Float`.
fn as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// `actual in expected`, where `expected` must be a list.
fn membership(actual: &Value, expected: &Value) -> Result<bool> {
    match expected {
        Value::List(items) => Ok(items.iter().any(|item| values_equal(actual, item))),
        other => bail!("operator `in`/`not_in` requires a list value, got `{other}`"),
    }
}

/// `actual contains expected`: list membership, or substring when `actual` is a string.
fn contains(actual: &Value, expected: &Value) -> Result<bool> {
    match actual {
        Value::List(items) => Ok(items.iter().any(|item| values_equal(item, expected))),
        Value::String(haystack) => match expected {
            Value::String(needle) => Ok(haystack.contains(needle.as_str())),
            other => bail!("operator `contains` on a string requires a string, got `{other}`"),
        },
        other => bail!("operator `contains` requires a list or string, got `{other}`"),
    }
}

/// `actual matches expected`, where `expected` is a regular expression and `actual` is a string.
fn matches_regex(actual: &Value, expected: &Value) -> Result<bool> {
    let haystack = match actual {
        Value::String(s) => s,
        other => bail!("operator `matches` requires a string value, got `{other}`"),
    };
    let pattern = match expected {
        Value::String(s) => s,
        other => bail!("operator `matches` requires a string pattern, got `{other}`"),
    };
    let re = regex::Regex::new(pattern)
        .with_context(|| format!("invalid regular expression `{pattern}`"))?;
    Ok(re.is_match(haystack))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stub resolver backed by a `(subject, metric) -> Value` map.
    #[derive(Default)]
    struct StubResolver {
        values: HashMap<(String, String), Value>,
    }

    impl StubResolver {
        fn with(mut self, subject: &str, metric: &str, value: Value) -> Self {
            self.values
                .insert((subject.to_string(), metric.to_string()), value);
            self
        }
    }

    impl MetricResolver for StubResolver {
        fn resolve(&mut self, subject: &str, _path: &Path, metric: &str) -> Result<Value> {
            self.values
                .get(&(subject.to_string(), metric.to_string()))
                .cloned()
                .ok_or_else(|| anyhow!("no stub value for {subject}.{metric}"))
        }
    }

    fn inputs(pairs: &[(&str, &str)]) -> HashMap<String, PathBuf> {
        pairs
            .iter()
            .map(|(name, path)| (name.to_string(), PathBuf::from(path)))
            .collect()
    }

    fn assertion(subject: &str, metric: &str, op: Operator, expected: Value) -> Assertion {
        Assertion {
            subject: subject.into(),
            metric: metric.into(),
            op,
            expected: Some(expected),
        }
    }

    // --- comparator unit tests -------------------------------------------------

    #[test]
    fn numeric_comparisons_with_int_float_coercion() {
        assert!(compare(&Value::Integer(5), Operator::Gt, &Value::Integer(3)).unwrap());
        assert!(compare(&Value::Integer(5), Operator::Ge, &Value::Float(5.0)).unwrap());
        assert!(compare(&Value::Float(2.5), Operator::Lt, &Value::Integer(3)).unwrap());
        assert!(!compare(&Value::Integer(1), Operator::Gt, &Value::Integer(2)).unwrap());
    }

    #[test]
    fn equality_and_inequality() {
        assert!(compare(&Value::Bool(true), Operator::Eq, &Value::Bool(true)).unwrap());
        assert!(
            compare(
                &Value::String("coord".into()),
                Operator::Ne,
                &Value::String("name".into())
            )
            .unwrap()
        );
        assert!(compare(&Value::Integer(7), Operator::Eq, &Value::Float(7.0)).unwrap());
    }

    #[test]
    fn membership_in_and_not_in() {
        let list = Value::List(vec![
            Value::String("coordinate".into()),
            Value::String("queryname".into()),
        ]);
        assert!(compare(&Value::String("coordinate".into()), Operator::In, &list).unwrap());
        assert!(compare(&Value::String("unsorted".into()), Operator::NotIn, &list).unwrap());
    }

    #[test]
    fn contains_on_list_and_string() {
        let contigs = Value::List(vec![
            Value::String("chr1".into()),
            Value::String("chr2".into()),
        ]);
        assert!(compare(&contigs, Operator::Contains, &Value::String("chr1".into())).unwrap());
        assert!(!compare(&contigs, Operator::Contains, &Value::String("chrX".into())).unwrap());

        let encoding = Value::String("illumina phred+33".into());
        assert!(
            compare(
                &encoding,
                Operator::Contains,
                &Value::String("phred+33".into())
            )
            .unwrap()
        );
    }

    #[test]
    fn matches_regex_operator() {
        let actual = Value::String("phred+33".into());
        assert!(
            compare(
                &actual,
                Operator::Matches,
                &Value::String(r"phred\+\d+".into())
            )
            .unwrap()
        );
        assert!(
            !compare(
                &actual,
                Operator::Matches,
                &Value::String(r"^solexa".into())
            )
            .unwrap()
        );
    }

    #[test]
    fn invalid_regex_is_an_error() {
        let err = compare(
            &Value::String("x".into()),
            Operator::Matches,
            &Value::String("(".into()),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("invalid regular expression"),
            "got: {err}"
        );
    }

    #[test]
    fn type_mismatch_on_ordering_is_an_error() {
        let err =
            compare(&Value::String("x".into()), Operator::Gt, &Value::Integer(1)).unwrap_err();
        assert!(
            err.to_string().contains("requires a numeric value"),
            "got: {err}"
        );
    }

    // --- engine tests with a stub resolver ------------------------------------

    #[test]
    fn evaluates_pass_and_fail() {
        let inputs = inputs(&[("bam", "sample.bam")]);
        let mut resolver = StubResolver::default()
            .with("bam", "read_count", Value::Integer(5321))
            .with("bam", "sort_order", Value::String("unknown".into()));
        let assertions = vec![
            assertion("bam", "read_count", Operator::Gt, Value::Integer(1000)),
            assertion(
                "bam",
                "sort_order",
                Operator::Eq,
                Value::String("coordinate".into()),
            ),
        ];

        let report = evaluate(&assertions, &inputs, &mut resolver, true).unwrap();
        assert_eq!(report.passed(), 1);
        assert_eq!(report.failed(), 1);
        assert_eq!(report.results[0].status, Status::Pass);
        assert!(
            report.results[0]
                .message
                .contains("read_count=5321 gt 1000")
        );
        assert_eq!(report.results[1].status, Status::Fail);
        assert!(report.results[1].message.contains("expected eq coordinate"));
    }

    #[test]
    fn boolean_metric_evaluates() {
        let inputs = inputs(&[("bam", "sample.bam")]);
        let mut resolver = StubResolver::default().with("bam", "has_index", Value::Bool(true));
        let assertions = vec![assertion(
            "bam",
            "has_index",
            Operator::Eq,
            Value::Bool(true),
        )];
        let report = evaluate(&assertions, &inputs, &mut resolver, false).unwrap();
        assert!(report.is_success());
    }

    #[test]
    fn cross_subject_reference_compares_same_metric() {
        let inputs = inputs(&[("read1", "R1.fq.gz"), ("read2", "R2.fq.gz")]);
        let mut resolver = StubResolver::default()
            .with("read1", "read_count", Value::Integer(1000))
            .with("read2", "read_count", Value::Integer(1000));
        // `read1 read_count eq read2` → parser stores expected as String("read2").
        let assertions = vec![assertion(
            "read1",
            "read_count",
            Operator::Eq,
            Value::String("read2".into()),
        )];
        let report = evaluate(&assertions, &inputs, &mut resolver, false).unwrap();
        assert!(report.is_success());
    }

    #[test]
    fn cross_subject_mismatch_fails() {
        let inputs = inputs(&[("read1", "R1.fq.gz"), ("read2", "R2.fq.gz")]);
        let mut resolver = StubResolver::default()
            .with("read1", "read_count", Value::Integer(1000))
            .with("read2", "read_count", Value::Integer(999));
        let assertions = vec![assertion(
            "read1",
            "read_count",
            Operator::Eq,
            Value::String("read2".into()),
        )];
        let report = evaluate(&assertions, &inputs, &mut resolver, false).unwrap();
        assert_eq!(report.failed(), 1);
    }

    #[test]
    fn unbound_subject_is_an_error() {
        let inputs = inputs(&[]);
        let mut resolver = StubResolver::default();
        let assertions = vec![assertion(
            "bam",
            "read_count",
            Operator::Gt,
            Value::Integer(1),
        )];
        let err = evaluate(&assertions, &inputs, &mut resolver, false).unwrap_err();
        assert!(err.to_string().contains("is not bound"), "got: {err}");
    }

    #[test]
    fn resolver_error_is_propagated() {
        let inputs = inputs(&[("bam", "sample.bam")]);
        let mut resolver = StubResolver::default(); // no value registered
        let assertions = vec![assertion(
            "bam",
            "read_count",
            Operator::Gt,
            Value::Integer(1),
        )];
        let err = evaluate(&assertions, &inputs, &mut resolver, false).unwrap_err();
        assert!(
            err.to_string().contains("computing bam.read_count"),
            "got: {err}"
        );
    }

    #[test]
    fn fail_fast_stops_after_first_failure() {
        let inputs = inputs(&[("bam", "sample.bam")]);
        let mut resolver = StubResolver::default()
            .with("bam", "read_count", Value::Integer(1))
            .with("bam", "mapped_reads", Value::Integer(1));
        let assertions = vec![
            assertion("bam", "read_count", Operator::Gt, Value::Integer(1000)), // fails
            assertion("bam", "mapped_reads", Operator::Gt, Value::Integer(0)),  // would pass
        ];
        let report = evaluate(&assertions, &inputs, &mut resolver, false).unwrap();
        assert_eq!(
            report.results.len(),
            1,
            "fail-fast should stop after first failure"
        );
        assert_eq!(report.failed(), 1);
    }

    #[test]
    fn continue_evaluates_all() {
        let inputs = inputs(&[("bam", "sample.bam")]);
        let mut resolver = StubResolver::default()
            .with("bam", "read_count", Value::Integer(1))
            .with("bam", "mapped_reads", Value::Integer(1));
        let assertions = vec![
            assertion("bam", "read_count", Operator::Gt, Value::Integer(1000)), // fails
            assertion("bam", "mapped_reads", Operator::Gt, Value::Integer(0)),  // passes
        ];
        let report = evaluate(&assertions, &inputs, &mut resolver, true).unwrap();
        assert_eq!(report.results.len(), 2);
        assert_eq!(report.passed(), 1);
        assert_eq!(report.failed(), 1);
    }
}
