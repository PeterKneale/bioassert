use crate::assertion::Assertion;
use std::fmt::{self, Display, Formatter};

/// The outcome of evaluating a single assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail,
    Error,
}

impl Outcome {
    /// The keyword that labels this outcome in the report and on the console.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Pass => "PASS",
            Outcome::Fail => "FAIL",
            Outcome::Error => "ERROR",
        }
    }
}

/// One evaluated assertion: the raw assertion exactly as written, the human-readable
/// message produced while evaluating it (the comparison detail, or the error), and the
/// resulting outcome. This is the unit of the [`AssertionReport`] and carries no
/// application-logging or presentation concerns.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub assertion: Assertion,
    pub message: String,
    pub outcome: Outcome,
}

impl AssertionResult {
    /// The plain report line for this result, e.g. `PASS. Expected ... got ...`.
    pub fn line(&self) -> String {
        format!("{}. {}", self.outcome.label(), self.message)
    }
}

impl Display for AssertionResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.line())
    }
}

/// The results of every assertion evaluated in a run. The engine returns this, and the
/// assertion report (assertions.log) is rendered from it. It is deliberately free of
/// any I/O, logging or color/icon concerns: those belong to the caller.
#[derive(Debug, Clone, Default)]
pub struct AssertionReport {
    results: Vec<AssertionResult>,
}

impl AssertionReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, result: AssertionResult) {
        self.results.push(result);
    }

    pub fn results(&self) -> &[AssertionResult] {
        &self.results
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Number of results with the given outcome.
    pub fn count(&self, outcome: Outcome) -> usize {
        self.results.iter().filter(|r| r.outcome == outcome).count()
    }

    pub fn has_failures(&self) -> bool {
        self.results.iter().any(|r| r.outcome == Outcome::Fail)
    }

    pub fn has_errors(&self) -> bool {
        self.results.iter().any(|r| r.outcome == Outcome::Error)
    }

    /// Renders the report as plain text, one line per assertion. This is the exact
    /// content written to the assertion report file (assertions.log).
    pub fn render(&self) -> String {
        let mut out = String::new();
        for result in &self.results {
            out.push_str(&result.line());
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(outcome: Outcome, message: &str) -> AssertionResult {
        AssertionResult {
            assertion: Assertion {
                file: "f".into(),
                metric: "m".into(),
                comparator: "eq".into(),
                expected: "x".into(),
            },
            message: message.into(),
            outcome,
        }
    }

    #[test]
    fn line_prefixes_the_outcome_label() {
        assert_eq!(result(Outcome::Pass, "ok").line(), "PASS. ok");
        assert_eq!(result(Outcome::Fail, "no").line(), "FAIL. no");
        assert_eq!(result(Outcome::Error, "boom").line(), "ERROR. boom");
    }

    #[test]
    fn render_writes_one_line_per_result() {
        let mut report = AssertionReport::new();
        report.push(result(Outcome::Pass, "a"));
        report.push(result(Outcome::Fail, "b"));
        assert_eq!(report.render(), "PASS. a\nFAIL. b\n");
    }

    #[test]
    fn counts_and_flags_reflect_outcomes() {
        let mut report = AssertionReport::new();
        report.push(result(Outcome::Pass, "a"));
        report.push(result(Outcome::Fail, "b"));
        report.push(result(Outcome::Error, "c"));
        assert_eq!(report.count(Outcome::Pass), 1);
        assert_eq!(report.count(Outcome::Fail), 1);
        assert_eq!(report.count(Outcome::Error), 1);
        assert!(report.has_failures());
        assert!(report.has_errors());
    }
}
