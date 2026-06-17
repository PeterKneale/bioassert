use crate::assertions::values::Value::BytesValue;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Value {
    BooleanValue(bool),
    BytesValue(u64),
    IntegerValue(u64),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // equality is only defined between the same variant — a byte count of 1 is not equal to an integer 1
            (Self::BooleanValue(a), Self::BooleanValue(b)) => a == b,
            (Self::BytesValue(a), Self::BytesValue(b)) => a == b,
            (Self::IntegerValue(a), Self::IntegerValue(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            // ordering is only meaningful for numeric types; cross-variant and boolean comparisons return None
            (Self::BytesValue(a), Self::BytesValue(b)) => a.partial_cmp(b),
            (Self::IntegerValue(a), Self::IntegerValue(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BooleanValue(value) => write!(f, "{value}"),
            Self::BytesValue(value) => write!(f, "{}", format_bytes(*value)),
            Self::IntegerValue(value) => write!(f, "{value}"),
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[(&str, u64)] = &[
        ("TB", 1024_u64.pow(4)),
        ("GB", 1024_u64.pow(3)),
        ("MB", 1024_u64.pow(2)),
        ("KB", 1024_u64),
    ];
    for (suffix, size) in UNITS {
        if bytes >= *size {
            return format!("{:.2}{}", bytes as f64 / *size as f64, suffix);
        }
    }
    format!("{}B", bytes)
}

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

pub fn parse_bytes(value: &str) -> Result<Value, ValueParseError> {
    let value = value.trim().to_uppercase();
    let units = [
        ("PB", 1024_u64.pow(5)),
        ("TB", 1024_u64.pow(4)),
        ("GB", 1024_u64.pow(3)),
        ("MB", 1024_u64.pow(2)),
        ("KB", 1024_u64),
        ("B", 1_u64),
    ];

    for (suffix, multiplier) in units {
        if let Some(number) = value.strip_suffix(suffix) {
            let integer = number
                .trim()
                .parse::<u64>()
                .map_err(|_| ValueParseError::InvalidBytes(value.clone()))?;
            let bytes = integer
                .checked_mul(multiplier)
                .ok_or_else(|| ValueParseError::InvalidBytes(value.clone()))?;
            return Ok(BytesValue(bytes));
        }
    }
    Err(ValueParseError::InvalidBytes(format!("Unknown format: {}", value)))
}
pub fn parse_integer(value: &str) -> Result<Value, ValueParseError> {
    let value = value.trim();
    if let Ok(v) = value.parse::<u64>() {
        return Ok(Value::IntegerValue(v));
    }
    Err(ValueParseError::InvalidInteger(value.to_string()))
}
pub fn parse_boolean(value: &str) -> Result<Value, ValueParseError> {
    let parsed = bool::from_str(value.trim());
    if parsed.is_ok() {
        return Ok(Value::BooleanValue(parsed.unwrap()));
    }
    Err(ValueParseError::InvalidBoolean(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // parse_bytes

    #[test]
    fn parse_bytes_parses_bytes() {
        assert_eq!(parse_bytes("100B").unwrap(), Value::BytesValue(100));
    }

    #[test]
    fn parse_bytes_parses_kilobytes() {
        assert_eq!(parse_bytes("1KB").unwrap(), Value::BytesValue(1024));
    }

    #[test]
    fn parse_bytes_parses_megabytes() {
        assert_eq!(parse_bytes("1MB").unwrap(), Value::BytesValue(1024 * 1024));
    }

    #[test]
    fn parse_bytes_parses_gigabytes() {
        assert_eq!(parse_bytes("1GB").unwrap(), Value::BytesValue(1024 * 1024 * 1024));
    }

    #[test]
    fn parse_bytes_is_case_insensitive() {
        assert_eq!(parse_bytes("1kb").unwrap(), Value::BytesValue(1024));
    }

    #[test]
    fn parse_bytes_trims_whitespace() {
        assert_eq!(parse_bytes(" 1KB ").unwrap(), Value::BytesValue(1024));
    }

    #[test]
    fn parse_bytes_rejects_bare_number() {
        assert!(matches!(parse_bytes("1024"), Err(ValueParseError::InvalidBytes(_))));
    }

    #[test]
    fn parse_bytes_rejects_non_numeric_prefix() {
        assert!(matches!(parse_bytes("abcKB"), Err(ValueParseError::InvalidBytes(_))));
    }

    // parse_integer

    #[test]
    fn parse_integer_parses_number() {
        assert_eq!(parse_integer("42").unwrap(), Value::IntegerValue(42));
    }

    #[test]
    fn parse_integer_parses_zero() {
        assert_eq!(parse_integer("0").unwrap(), Value::IntegerValue(0));
    }

    #[test]
    fn parse_integer_trims_whitespace() {
        assert_eq!(parse_integer(" 42 ").unwrap(), Value::IntegerValue(42));
    }

    #[test]
    fn parse_integer_rejects_text() {
        assert!(matches!(parse_integer("abc"), Err(ValueParseError::InvalidInteger(_))));
    }

    #[test]
    fn parse_integer_rejects_float() {
        assert!(matches!(parse_integer("1.5"), Err(ValueParseError::InvalidInteger(_))));
    }

    // parse_boolean

    #[test]
    fn parse_boolean_parses_true() {
        assert_eq!(parse_boolean("true").unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn parse_boolean_parses_false() {
        assert_eq!(parse_boolean("false").unwrap(), Value::BooleanValue(false));
    }

    #[test]
    fn parse_boolean_trims_whitespace() {
        assert_eq!(parse_boolean(" true ").unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn parse_boolean_rejects_yes() {
        assert!(matches!(parse_boolean("yes"), Err(ValueParseError::InvalidBoolean(_))));
    }

    // PartialEq

    #[test]
    fn eq_same_variant_same_value() {
        assert_eq!(Value::BytesValue(100), Value::BytesValue(100));
        assert_eq!(Value::IntegerValue(5), Value::IntegerValue(5));
        assert_eq!(Value::BooleanValue(true), Value::BooleanValue(true));
    }

    #[test]
    fn eq_same_variant_different_value() {
        assert_ne!(Value::BytesValue(100), Value::BytesValue(200));
        assert_ne!(Value::IntegerValue(1), Value::IntegerValue(2));
        assert_ne!(Value::BooleanValue(true), Value::BooleanValue(false));
    }

    #[test]
    fn eq_cross_variant_is_false() {
        assert_ne!(Value::BytesValue(1), Value::IntegerValue(1));
        assert_ne!(Value::BytesValue(0), Value::BooleanValue(false));
    }

    // PartialOrd

    #[test]
    fn ord_bytes_less_than() {
        assert!(Value::BytesValue(100) < Value::BytesValue(200));
    }

    #[test]
    fn ord_bytes_greater_than() {
        assert!(Value::BytesValue(200) > Value::BytesValue(100));
    }

    #[test]
    fn ord_integer_less_than() {
        assert!(Value::IntegerValue(1) < Value::IntegerValue(2));
    }

    #[test]
    fn ord_boolean_returns_none() {
        assert_eq!(Value::BooleanValue(true).partial_cmp(&Value::BooleanValue(false)), None);
    }

    #[test]
    fn ord_cross_variant_returns_none() {
        assert_eq!(Value::BytesValue(1).partial_cmp(&Value::IntegerValue(1)), None);
    }
}
