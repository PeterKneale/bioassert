//! Process exit codes.
//!
//! Spec: `docs/spec.md` → "CLI Design → Exit codes".
//!
//! * `0` – All assertions passed.
//! * `1` – One or more assertions failed.
//! * `2` – CLI or configuration error (e.g. parse error).

/// All assertions passed.
pub const SUCCESS: i32 = 0;

/// One or more assertions failed.
pub const ASSERTION_FAILED: i32 = 1;

/// CLI or configuration error (bad args, unreadable/parse error, etc.).
pub const USAGE_ERROR: i32 = 2;
