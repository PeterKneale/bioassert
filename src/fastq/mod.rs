//! FASTQ metric executors. Each `fastq.*` metric streams a FASTQ file once (plain or gzipped,
//! see [`functions::scan`]) and asserts on what the pass found. The file is never held in
//! memory: a read set is tens of millions of records, so only a fixed-size summary is kept.

pub mod functions;

mod valid;

pub use valid::FastqValidExecutor;
