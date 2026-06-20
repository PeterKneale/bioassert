use crate::core::comparisons::Comparator;
use std::path::PathBuf;

pub struct AssertionRequest {
    pub file: PathBuf,
    pub comparator: Comparator,
    pub expected: String,
}