#[derive(Debug, Clone)]
pub struct Assertion {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
    /// An optional guard that must hold for the assertion to run. When the guard is
    /// not satisfied the assertion is skipped rather than passed or failed.
    pub guard: Option<Guard>,
}

/// A guard clause introduced by `if` (run when the condition holds) or `unless` (run
/// when it does not). `negate` is `true` for `unless`.
#[derive(Debug, Clone)]
pub struct Guard {
    pub negate: bool,
    pub condition: Condition,
}

/// The condition a guard tests. It is a full assertion in its own right (file, metric,
/// comparator and expected value) and is evaluated by the same executor chain. The
/// shorthand form (`if file.exists`) is expanded by the parser into a condition on the
/// assertion's own file with `eq true`.
#[derive(Debug, Clone)]
pub struct Condition {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
}
