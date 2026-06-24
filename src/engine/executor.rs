use crate::bam::{
    BamCountExecutor, BamHeaderFieldExecutor, BamReadGroupPresentExecutor, BamReadGroupTagExecutor,
};
use crate::core::{AssertionExecutor, AssertionRequest, BioAssertError, Comparator, Value};
use crate::delimited::{
    DelimitedCellExecutor, DelimitedColumnAllExecutor, DelimitedColumnCountExecutor,
    DelimitedLineCountExecutor,
};
use crate::engine::assertion::Assertion;
use crate::engine::report::{AssertionReport, AssertionResult, Outcome};
use crate::fasta::{FastaCountExecutor, FastaSequenceFieldExecutor, FastaSequencePresentExecutor};
use crate::file::{
    FileCompressedExecutor, FileCompressionExecutor, FileEmptyExecutor, FileExistsExecutor,
    FileLinesExecutor, FileSizeExecutor,
};
use crate::text::{TextLengthExecutor, TextValueExecutor};

/// Evaluates every assertion and collects the outcomes into an [`AssertionReport`].
/// This is the structure callers turn into the assertion report (assertions.log).
pub fn execute_all(assertions: impl IntoIterator<Item = Assertion>) -> AssertionReport {
    let mut report = AssertionReport::new();
    for assertion in assertions {
        report.push(execute(assertion));
    }
    report
}

/// The result of evaluating an assertion before it is labelled with an [`Outcome`]:
/// either it ran (and we know whether it held) or it was skipped because its guard was
/// not satisfied. An evaluation error is reported separately as [`Outcome::Error`].
enum Evaluation {
    Ran { success: bool, message: String },
    Skipped { message: String },
}

/// Evaluates a single assertion, capturing the outcome (PASS/FAIL/ERROR/SKIP) and the
/// human-readable message rather than returning a bare bool or error. An evaluation
/// error is captured as [`Outcome::Error`] so the caller always gets a complete result.
pub fn execute(assertion: Assertion) -> AssertionResult {
    match evaluate(&assertion) {
        Ok(Evaluation::Ran { success, message }) => AssertionResult {
            assertion,
            message,
            outcome: if success {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
        },
        Ok(Evaluation::Skipped { message }) => AssertionResult {
            assertion,
            message,
            outcome: Outcome::Skip,
        },
        Err(error) => AssertionResult {
            message: error.to_string(),
            assertion,
            outcome: Outcome::Error,
        },
    }
}

/// Runs an assertion's guard (if any) and then the assertion itself. A guard that is
/// not satisfied yields [`Evaluation::Skipped`]; a guard that cannot be evaluated yields
/// an [`Err`] wrapped as [`BioAssertError::Guard`] so the report attributes it to the
/// guard rather than the main metric.
fn evaluate(assertion: &Assertion) -> Result<Evaluation, BioAssertError> {
    if let Some(guard) = &assertion.guard {
        let c = &guard.condition;
        let (held, actual, comparator) = run_metric(&c.file, &c.metric, &c.comparator, &c.expected)
            .map_err(|e| BioAssertError::Guard(Box::new(e)))?;
        // `unless` inverts the condition: the assertion runs when the condition does not hold.
        let active = held ^ guard.negate;
        if !active {
            let reason = if guard.negate {
                "condition met"
            } else {
                "condition not met"
            };
            return Ok(Evaluation::Skipped {
                message: format!(
                    "Skipped ({}): {} {} {} {}, got {}",
                    reason, c.file, c.metric, comparator, c.expected, actual
                ),
            });
        }
    }

    let (success, actual, comparator) = run_metric(
        &assertion.file,
        &assertion.metric,
        &assertion.comparator,
        &assertion.expected,
    )?;
    Ok(Evaluation::Ran {
        success,
        message: format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, comparator, assertion.expected, actual
        ),
    })
}

/// Runs a single metric against a resource: builds the request, finds the matching
/// executor and returns whether the comparison held, the actual value and the parsed
/// comparator (the comparator is returned so the caller can render it in its symbolic
/// form). Shared by the main assertion and by guard conditions, so any metric can guard
/// any other.
fn run_metric(
    resource: &str,
    metric: &str,
    comparator: &str,
    expected: &str,
) -> Result<(bool, Value, Comparator), BioAssertError> {
    let request = AssertionRequest {
        // Strip the locator's quotes once here, so every executor receives a clean locator
        // and a quoted path/literal containing spaces (e.g. 'my output.tsv') resolves too.
        locator: crate::core::strip_quotes(resource).to_string(),
        comparator: comparator.parse()?,
        expected: expected.to_string(),
    };
    let (success, actual) = dispatch(metric, &request)?;
    Ok((success, actual, request.comparator))
}

/// Finds the executor matching `metric` and runs it, returning whether the comparison
/// held and the actual value. Dispatch is first-match-wins.
fn dispatch(metric: &str, request: &AssertionRequest) -> Result<(bool, Value), BioAssertError> {
    if let Some(e) = FileExistsExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FileSizeExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FileEmptyExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FileLinesExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FileCompressionExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FileCompressedExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = DelimitedColumnCountExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = DelimitedLineCountExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = DelimitedCellExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = DelimitedColumnAllExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = BamCountExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = BamHeaderFieldExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = BamReadGroupPresentExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = BamReadGroupTagExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FastaCountExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FastaSequenceFieldExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = FastaSequencePresentExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = TextValueExecutor::try_parse(metric) {
        return run(e, request);
    }
    if let Some(e) = TextLengthExecutor::try_parse(metric) {
        return run(e, request);
    }
    Err(BioAssertError::Metric(metric.to_string()))
}

fn run<E: AssertionExecutor>(
    executor: E,
    request: &AssertionRequest,
) -> Result<(bool, Value), BioAssertError> {
    let result = executor.execute(request)?;
    Ok((result.success, result.actual))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::assertion::{Condition, Guard};

    fn assertion(
        file: &str,
        metric: &str,
        comparator: &str,
        expected: &str,
        guard: Option<Guard>,
    ) -> Assertion {
        Assertion {
            file: file.to_string(),
            metric: metric.to_string(),
            comparator: comparator.to_string(),
            expected: expected.to_string(),
            guard,
        }
    }

    fn bool_guard(negate: bool, file: &str, metric: &str) -> Guard {
        Guard {
            negate,
            condition: Condition {
                file: file.to_string(),
                metric: metric.to_string(),
                comparator: "eq".to_string(),
                expected: "true".to_string(),
            },
        }
    }

    #[test]
    fn guard_satisfied_runs_the_assertion() {
        let guard = bool_guard(false, "tests/data/example.tsv", "file.exists");
        let result = execute(assertion(
            "tests/data/example.tsv",
            "tsv.columns.count",
            "eq",
            "3",
            Some(guard),
        ));
        assert_eq!(result.outcome, Outcome::Pass);
    }

    #[test]
    fn guard_satisfied_can_still_fail() {
        let guard = bool_guard(false, "tests/data/example.tsv", "file.exists");
        let result = execute(assertion(
            "tests/data/example.tsv",
            "tsv.columns.count",
            "eq",
            "99",
            Some(guard),
        ));
        assert_eq!(result.outcome, Outcome::Fail);
    }

    #[test]
    fn guard_not_satisfied_skips() {
        // file.exists on a missing file is false, so the guard is not satisfied
        let guard = bool_guard(false, "tests/data/missing.tsv", "file.exists");
        let result = execute(assertion(
            "tests/data/missing.tsv",
            "tsv.columns.count",
            "eq",
            "3",
            Some(guard),
        ));
        assert_eq!(result.outcome, Outcome::Skip);
    }

    #[test]
    fn unless_skips_when_the_condition_holds() {
        // empty_file.txt is empty, so file.empty holds and `unless` skips
        let guard = bool_guard(true, "tests/data/empty_file.txt", "file.empty");
        let result = execute(assertion(
            "tests/data/empty_file.txt",
            "file.lines",
            "gt",
            "0",
            Some(guard),
        ));
        assert_eq!(result.outcome, Outcome::Skip);
    }

    #[test]
    fn guard_that_errors_is_reported_as_error() {
        let guard = Guard {
            negate: false,
            condition: Condition {
                file: "tests/data/missing.tsv".to_string(),
                metric: "file.size".to_string(),
                comparator: "gt".to_string(),
                expected: "0B".to_string(),
            },
        };
        let result = execute(assertion(
            "tests/data/example.tsv",
            "tsv.lines.count",
            "gt",
            "0",
            Some(guard),
        ));
        assert_eq!(result.outcome, Outcome::Error);
        assert!(
            result.message.contains("guard could not be evaluated"),
            "message was: {}",
            result.message
        );
    }

    #[test]
    fn guard_with_a_non_boolean_value_errors() {
        // a numeric metric cannot be compared against `eq true`
        let guard = bool_guard(false, "tests/data/example.tsv", "tsv.columns.count");
        let result = execute(assertion(
            "tests/data/example.tsv",
            "tsv.lines.count",
            "gt",
            "0",
            Some(guard),
        ));
        assert_eq!(result.outcome, Outcome::Error);
    }

    #[test]
    fn assertion_without_a_guard_runs_normally() {
        let result = execute(assertion(
            "tests/data/example.tsv",
            "tsv.columns.count",
            "eq",
            "3",
            None,
        ));
        assert_eq!(result.outcome, Outcome::Pass);
    }
}
