use crate::comparisons::Comparator;
use crate::errors::BioAssertError;
use crate::metrics::{
    DelimitedCellExecutor, DelimitedColumnCountExecutor, DelimitedLineCountExecutor,
    FileEmptyExecutor, FileExistsExecutor, FileLinesExecutor, FileSizeExecutor, MetricExecutor,
};
use crate::parser::Assertion;

pub fn execute(assertion: Assertion) -> Result<bool, BioAssertError> {
    if let Some(e) = FileExistsExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    if let Some(e) = FileSizeExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    if let Some(e) = FileEmptyExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    if let Some(e) = FileLinesExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    if let Some(e) = DelimitedColumnCountExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    if let Some(e) = DelimitedLineCountExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    if let Some(e) = DelimitedCellExecutor::try_parse(&assertion.metric) { return dispatch(e, &assertion); }
    Err(BioAssertError::Metric(assertion.metric))
}

fn dispatch<E: MetricExecutor>(executor: E, assertion: &Assertion) -> Result<bool, BioAssertError> {
    let comparator = assertion.comparator.parse::<Comparator>()?;
    let result = executor.execute(assertion)?;
    let message = format!(
        "Expected {} {} {} {}, got {}",
        assertion.file, assertion.metric, comparator, assertion.expected, result.actual
    );
    if result.success {
        println!("PASS. {}", message);
    } else {
        println!("FAIL. {}", message);
    }
    Ok(result.success)
}
