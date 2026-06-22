//! FASTA sequence metric executors. Each `fasta.*` metric reads a FASTA file's records
//! (scanned once and cached as a per-record digest, see [`functions::read_records`]) and
//! asserts a count, a total length, a per-record name / description / length, or a presence
//! flag. Per-record metrics live under `fasta.seq.*` and the whole-file aggregate under
//! `fasta.length`, leaving room for future index metrics (e.g. `fasta.index.*`).

pub mod functions;

mod count;
mod sequence;

pub use count::FastaCountExecutor;
pub use sequence::{FastaSequenceFieldExecutor, FastaSequencePresentExecutor};
