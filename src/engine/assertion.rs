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
/// comparator and expected value), written in the same `resource metric comparator value`
/// form as the assertion it guards, and is evaluated by the same executor chain. There is
/// no shorthand form: the resource and comparator are always stated explicitly.
#[derive(Debug, Clone)]
pub struct Condition {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
}
