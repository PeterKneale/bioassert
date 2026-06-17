mod delimited_cell;
mod delimited_column_count;
mod delimited_line_count;
mod delimited_utils;
mod file_empty;
mod file_exists;
mod file_lines;
mod file_size;

pub use delimited_cell::DelimitedCellExecutor;
pub use delimited_column_count::DelimitedColumnCountExecutor;
pub use delimited_line_count::DelimitedLineCountExecutor;
pub use file_empty::FileEmptyExecutor;
pub use file_exists::FileExistsExecutor;
pub use file_lines::FileLinesExecutor;
pub use file_size::FileSizeExecutor;

use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;

pub struct ExecutionResult {
    pub success: bool,
    pub actual: Value,
}

pub trait MetricExecutor {
    fn try_parse(metric: &str) -> Option<Self>
    where
        Self: Sized;
    fn execute(self, assertion: &Assertion) -> Result<ExecutionResult, BioAssertError>;
}
