#[derive(Debug, Clone)]
pub struct Assertion {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
}
