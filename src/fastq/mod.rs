//! FASTQ metric executors. Each `fastq.*` metric streams a FASTQ file once (plain or gzipped,
//! see [`functions::scan`]) and asserts on what the pass found. The file is never held in
//! memory: a read set is tens of millions of records, so only a fixed-size summary is kept.
//! Read sets are assessed in aggregate only; there are no per-record metrics.

pub mod functions;

mod count;
mod quality;
mod read_length;
mod valid;

pub use count::FastqCountExecutor;
pub use quality::FastqQualityEncodingExecutor;
pub use read_length::FastqReadLengthExecutor;
pub use valid::FastqValidExecutor;
