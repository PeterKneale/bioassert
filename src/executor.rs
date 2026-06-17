use crate::assertions::{parse_metric, Metric};
use crate::metrics::{
    DelimitedCellExecutor, DelimitedColumnCountExecutor, DelimitedLineCountExecutor,
    FileEmptyExecutor, FileExistsExecutor, FileLinesExecutor, FileSizeExecutor, MetricExecutor,
};
use crate::parser::Assertion;

pub fn execute(assertion: Assertion) -> Result<bool, Box<dyn std::error::Error>> {
    let metric = parse_metric(assertion.metric.as_str())?;
    let (result, message) = match metric {
        Metric::FileExists => FileExistsExecutor.execute(assertion),
        Metric::FileSize => FileSizeExecutor.execute(assertion),
        Metric::FileEmpty => FileEmptyExecutor.execute(assertion),
        Metric::FileLines => FileLinesExecutor.execute(assertion),
        Metric::DelimitedColumnCount(d) => DelimitedColumnCountExecutor { delimiter: d }.execute(assertion),
        Metric::DelimitedLineCount(_) => DelimitedLineCountExecutor.execute(assertion),
        Metric::DelimitedCell(d, line, col) => DelimitedCellExecutor { delimiter: d, line, col }.execute(assertion),
    }?;
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
