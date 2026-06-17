use crate::assertions::BioAssertError;
use crate::metrics::{
    DelimitedCellExecutor, DelimitedColumnCountExecutor, DelimitedLineCountExecutor,
    FileEmptyExecutor, FileExistsExecutor, FileLinesExecutor, FileSizeExecutor, MetricExecutor,
};
use crate::parser::Assertion;

pub fn execute(assertion: Assertion) -> Result<bool, BioAssertError> {
    let metric = assertion.metric.as_str();

    if let Some(e) = FileExistsExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }
    if let Some(e) = FileSizeExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }
    if let Some(e) = FileEmptyExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }
    if let Some(e) = FileLinesExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }
    if let Some(e) = DelimitedColumnCountExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }
    if let Some(e) = DelimitedLineCountExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }
    if let Some(e) = DelimitedCellExecutor::try_parse(metric) {
        return dispatch(e, assertion);
    }

    Err(BioAssertError::Metric(metric.to_string()))
}

fn dispatch<E: MetricExecutor>(executor: E, assertion: Assertion) -> Result<bool, BioAssertError> {
    let (result, message) = executor.execute(assertion)?;
    announce(result, message);
    Ok(result)
}

fn announce(result: bool, message: String) {
    if result {
        println!("PASS. {}", message);
    } else {
        println!("FAIL. {}", message);
    }
}
