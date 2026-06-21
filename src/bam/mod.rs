//! BAM header metric executors. Each `bam.header.*` metric reads the SAM header of a BAM
//! file (parsed once and cached, see [`functions::read_header`]) and asserts a count, a
//! read-group tag value, a presence flag, or an `@HD` field. The `bam.header.*` namespace
//! leaves room for future record-level metrics (e.g. `bam.records.*`).

pub mod functions;

mod count;
mod header;
mod read_group;

pub use count::BamCountExecutor;
pub use header::BamHeaderFieldExecutor;
pub use read_group::{BamReadGroupPresentExecutor, BamReadGroupTagExecutor};
