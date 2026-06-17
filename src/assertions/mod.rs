mod delimited;
mod file;

pub use delimited::cell::DelimitedCellExecutor;
pub use delimited::column_count::DelimitedColumnCountExecutor;
pub use delimited::line_count::DelimitedLineCountExecutor;
pub use file::empty::FileEmptyExecutor;
pub use file::exists::FileExistsExecutor;
pub use file::lines::FileLinesExecutor;
pub use file::size::FileSizeExecutor;

use crate::errors::BioAssertError;
use crate::parser::Assertion;
use crate::values::Value;

pub struct AssertionExecutionResult {
    pub success: bool,
    pub actual: Value,
}

pub trait AssertionExecutor {
    fn try_parse(metric: &str) -> Option<Self>
    where
        Self: Sized;
    fn execute(self, assertion: &Assertion) -> Result<AssertionExecutionResult, BioAssertError>;
}
