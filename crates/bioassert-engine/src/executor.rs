use crate::assertion::Assertion;
use bioassert_core::{AssertionExecutor, AssertionRequest, BioAssertError};
use bioassert_delimited::{DelimitedCellExecutor, DelimitedColumnCountExecutor, DelimitedLineCountExecutor};
use bioassert_file::{FileEmptyExecutor, FileExistsExecutor, FileLinesExecutor, FileSizeExecutor};
use std::path::PathBuf;

pub fn execute(assertion: Assertion) -> Result<bool, BioAssertError> {
    tracing::debug!(file = %assertion.file, metric = %assertion.metric, comparator = %assertion.comparator, expected = %assertion.expected, "executing assertion");
    let request = AssertionRequest {
        file: PathBuf::from(&assertion.file),
        comparator: assertion.comparator.parse()?,
        expected: assertion.expected.clone(),
    };
    if let Some(e) = FileExistsExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    if let Some(e) = FileSizeExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    if let Some(e) = FileEmptyExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    if let Some(e) = FileLinesExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    if let Some(e) = DelimitedColumnCountExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    if let Some(e) = DelimitedLineCountExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    if let Some(e) = DelimitedCellExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion, request); }
    Err(BioAssertError::Metric(assertion.metric))
}

fn dispatch<E: AssertionExecutor>(executor: E, assertion: &Assertion, request: AssertionRequest) -> Result<bool, BioAssertError> {
    let result = executor.execute(&request)?;
    tracing::debug!(success = result.success, actual = %result.actual, "assertion result");
    let message = format!(
        "Expected {} {} {} {}, got {}",
        assertion.file, assertion.metric, request.comparator, request.expected, result.actual
    );
    if result.success {
        tracing::info!("PASS. {}", message);
    } else {
        tracing::info!("FAIL. {}", message);
    }
    Ok(result.success)
}
