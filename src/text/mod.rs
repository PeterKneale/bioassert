//! Inline text resource metric executors, under the `text.*` namespace.
//!
//! A `text` resource is the assertion's first token taken verbatim as a literal value
//! (after the central quote-stripping in `engine::executor`), rather than a path that is
//! opened. It has no I/O and cannot fail to "open", so its metrics produce PASS or FAIL
//! but never the "could not open" ERROR that file-backed families do. This makes a `text`
//! resource a safe guard input, though a guard comparing literals known at generation time
//! decides little the generator could not; the durable use is reading a runtime resource.
mod length;
mod value;
pub use length::TextLengthExecutor;
pub use value::TextValueExecutor;
