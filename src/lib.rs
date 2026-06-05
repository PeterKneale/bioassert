//! BioAssert - A bioinformatics assertion and validation library.
//!
//! This library provides functions for asserting and validating properties
//! of biological sequences and bioinformatics file formats.

use anyhow::{bail, Result};
use std::fmt::{Display, Formatter};

pub mod bam;
pub mod common;
pub mod parse;


pub fn parse_comparator(s: &str) -> Result<Comparator> {
    match s {
        "eq" => Ok(Comparator::Eq),
        "ne" => Ok(Comparator::Ne),
        "lt" => Ok(Comparator::Lt),
        "le" => Ok(Comparator::Le),
        "gt" => Ok(Comparator::Gt),
        "ge" => Ok(Comparator::Ge),
        _ => bail!("unsupported comparator: '{s}' (expected: eq, ne, lt, le, gt, ge)"),
    }
}

#[derive(Clone, Copy)]
pub enum Comparator {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Display for Comparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        };

        write!(f, "{s}")
    }
}

impl Comparator {
    pub fn compare(self, actual: u64, expected: u64) -> bool {
        match self {
            Self::Eq => actual == expected,
            Self::Ne => actual != expected,
            Self::Lt => actual < expected,
            Self::Le => actual <= expected,
            Self::Gt => actual > expected,
            Self::Ge => actual >= expected,
        }
    }
}
