//! Core data model for BioAssert.
//!
//! Implements the types described in `docs/spec.md` → "Architecture → Data Models".
//! These types are shared across the parser, engine, providers, and reporter.

use std::fmt;
use std::path::PathBuf;

/// A typed value produced by a metric provider or parsed from an assertion.
///
/// Spec: `enum Value { Bool, Integer, Float, String, List }`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Integer(u64),
    Float(f64),
    String(String),
    List(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::Integer(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::String(s) => write!(f, "{s}"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
        }
    }
}

/// A comparison operator.
///
/// Spec: `enum Operator { Eq, Ne, Gt, Lt, Ge, Le, In, NotIn, Contains, Matches }`.
/// The DSL spells `Ge`/`Le`/`NotIn` as `gte`/`lte`/`not_in` (see `from_dsl`/`as_dsl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    In,
    NotIn,
    Contains,
    Matches,
}

impl Operator {
    /// Parse an operator from its DSL token. Returns `None` for unknown tokens.
    pub fn from_dsl(token: &str) -> Option<Self> {
        Some(match token {
            "eq" => Operator::Eq,
            "ne" => Operator::Ne,
            "gt" => Operator::Gt,
            "lt" => Operator::Lt,
            "gte" => Operator::Ge,
            "lte" => Operator::Le,
            "in" => Operator::In,
            "not_in" => Operator::NotIn,
            "contains" => Operator::Contains,
            "matches" => Operator::Matches,
            _ => return None,
        })
    }

    /// The canonical DSL token for this operator.
    pub fn as_dsl(self) -> &'static str {
        match self {
            Operator::Eq => "eq",
            Operator::Ne => "ne",
            Operator::Gt => "gt",
            Operator::Lt => "lt",
            Operator::Ge => "gte",
            Operator::Le => "lte",
            Operator::In => "in",
            Operator::NotIn => "not_in",
            Operator::Contains => "contains",
            Operator::Matches => "matches",
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_dsl())
    }
}

/// A single parsed assertion: `<subject> <metric> <operator> <value>`.
///
/// `subject` is a *virtual* input name (e.g. `bam`) resolved via `--input name=path`.
#[derive(Debug, Clone, PartialEq)]
pub struct Assertion {
    pub subject: String,
    pub metric: String,
    pub op: Operator,
    pub expected: Option<Value>,
}

/// Outcome of evaluating one assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pass => f.write_str("PASS"),
            Status::Fail => f.write_str("FAIL"),
        }
    }
}

/// The recorded result of evaluating one assertion.
///
/// `subject` keeps the virtual name and `resolved_path` keeps the physical file, enabling the
/// `bam (sample.bam)` diagnostic format from the spec.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub subject: String,
    pub resolved_path: Option<PathBuf>,
    pub metric: String,
    pub op: Operator,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
    pub status: Status,
    pub message: String,
}

impl AssertionResult {
    /// Render the subject as `bam (sample.bam)` when the path is known, else just `bam`.
    pub fn subject_label(&self) -> String {
        match &self.resolved_path {
            Some(path) => format!("{} ({})", self.subject, path.display()),
            None => self.subject.clone(),
        }
    }
}

/// Aggregated results of a run.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub results: Vec<AssertionResult>,
}

impl Report {
    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == Status::Pass)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == Status::Fail)
            .count()
    }

    /// True when there are no failures.
    pub fn is_success(&self) -> bool {
        self.failed() == 0
    }

    /// Process exit code implied by the report (`0` success, `1` any failure).
    pub fn exit_code(&self) -> i32 {
        if self.is_success() {
            crate::exit::SUCCESS
        } else {
            crate::exit::ASSERTION_FAILED
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_round_trips_through_dsl() {
        let tokens = [
            "eq", "ne", "gt", "lt", "gte", "lte", "in", "not_in", "contains", "matches",
        ];
        for token in tokens {
            let op = Operator::from_dsl(token).expect("known token");
            assert_eq!(op.as_dsl(), token);
        }
    }

    #[test]
    fn unknown_operator_token_is_rejected() {
        assert!(Operator::from_dsl("exists").is_none());
        assert!(Operator::from_dsl(">").is_none());
    }

    #[test]
    fn value_display_formats_lists() {
        let v = Value::List(vec![Value::String("chr1".into()), Value::Integer(2)]);
        assert_eq!(v.to_string(), "[chr1, 2]");
    }

    #[test]
    fn report_counts_and_exit_code() {
        let mut report = Report::default();
        report.results.push(AssertionResult {
            subject: "bam".into(),
            resolved_path: Some(PathBuf::from("sample.bam")),
            metric: "exists".into(),
            op: Operator::Eq,
            expected: Some(Value::Bool(true)),
            actual: Some(Value::Bool(true)),
            status: Status::Pass,
            message: String::new(),
        });
        assert_eq!(report.passed(), 1);
        assert_eq!(report.failed(), 0);
        assert!(report.is_success());
        assert_eq!(report.exit_code(), crate::exit::SUCCESS);

        report.results.push(AssertionResult {
            subject: "bam".into(),
            resolved_path: Some(PathBuf::from("sample.bam")),
            metric: "sort_order".into(),
            op: Operator::Eq,
            expected: Some(Value::String("coordinate".into())),
            actual: Some(Value::String("unknown".into())),
            status: Status::Fail,
            message: "sort_order=unknown, expected 'coordinate'".into(),
        });
        assert_eq!(report.failed(), 1);
        assert!(!report.is_success());
        assert_eq!(report.exit_code(), crate::exit::ASSERTION_FAILED);
    }

    #[test]
    fn subject_label_includes_path_when_present() {
        let result = AssertionResult {
            subject: "bam".into(),
            resolved_path: Some(PathBuf::from("sample.bam")),
            metric: "exists".into(),
            op: Operator::Eq,
            expected: Some(Value::Bool(true)),
            actual: Some(Value::Bool(true)),
            status: Status::Pass,
            message: String::new(),
        };
        assert_eq!(result.subject_label(), "bam (sample.bam)");
    }
}
