use crate::engine::assertion::Assertion;
use crate::engine::report::{AssertionReport, AssertionResult, Outcome};
use crate::bam::{BamCountExecutor, BamHeaderFieldExecutor, BamReadGroupPresentExecutor, BamReadGroupTagExecutor};
use crate::core::{AssertionExecutor, AssertionRequest, BioAssertError};
use crate::delimited::{DelimitedCellExecutor, DelimitedColumnAllExecutor, DelimitedColumnCountExecutor, DelimitedLineCountExecutor};
use crate::fasta::{FastaCountExecutor, FastaSequenceFieldExecutor, FastaSequencePresentExecutor};
use crate::file::{FileEmptyExecutor, FileExistsExecutor, FileLinesExecutor, FileSizeExecutor};
use std::path::PathBuf;

/// Evaluates every assertion and collects the outcomes into an [`AssertionReport`].
/// This is the structure callers turn into the assertion report (assertions.log).
pub fn execute_all(assertions: impl IntoIterator<Item = Assertion>) -> AssertionReport {
    let mut report = AssertionReport::new();
    for assertion in assertions {
        report.push(execute(assertion));
    }
    report
}

/// Evaluates a single assertion, capturing the outcome (PASS/FAIL/ERROR) and the
/// human-readable message rather than returning a bare bool or error. An evaluation
/// error is captured as [`Outcome::Error`] so the caller always gets a complete result.
pub fn execute(assertion: Assertion) -> AssertionResult {
    match evaluate(&assertion) {
        Ok((success, message)) => AssertionResult {
            assertion,
            message,
            outcome: if success { Outcome::Pass } else { Outcome::Fail },
        },
        Err(error) => AssertionResult {
            message: error.to_string(),
            assertion,
            outcome: Outcome::Error,
        },
    }
}

/// Runs the assertion through the matching executor, returning whether it held and a
/// human-readable message describing the comparison.
fn evaluate(assertion: &Assertion) -> Result<(bool, String), BioAssertError> {
    let request = AssertionRequest {
        file: PathBuf::from(&assertion.file),
        comparator: assertion.comparator.parse()?,
        expected: assertion.expected.clone(),
    };
    if let Some(e) = FileExistsExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = FileSizeExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = FileEmptyExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = FileLinesExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = DelimitedColumnCountExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = DelimitedLineCountExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = DelimitedCellExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = DelimitedColumnAllExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = BamCountExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = BamHeaderFieldExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = BamReadGroupPresentExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = BamReadGroupTagExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = FastaCountExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = FastaSequenceFieldExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    if let Some(e) = FastaSequencePresentExecutor::try_parse(&assertion.metric) { return dispatch(e, assertion, request); }
    Err(BioAssertError::Metric(assertion.metric.clone()))
}

fn dispatch<E: AssertionExecutor>(executor: E, assertion: &Assertion, request: AssertionRequest) -> Result<(bool, String), BioAssertError> {
    let result = executor.execute(&request)?;
    let message = format!(
        "Expected {} {} {} {}, got {}",
        assertion.file, assertion.metric, request.comparator, request.expected, result.actual
    );
    Ok((result.success, message))
}
