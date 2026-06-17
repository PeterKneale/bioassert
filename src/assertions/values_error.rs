use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ValueParseError {
    InvalidBoolean(String),
    InvalidBytes(String),
    InvalidInteger(String),
}

impl Error for ValueParseError {}

impl Display for ValueParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBoolean(value) => write!(f, "Invalid boolean: {value}"),
            Self::InvalidBytes(value) => write!(f, "Invalid bytes: {value}"),
            Self::InvalidInteger(value) => write!(f, "Invalid integer: {value}"),
        }
    }
}
