use super::errors::ValueParseError;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum Value {
    BooleanValue(bool),
    BytesValue(u64),
    IntegerValue(u64),
    StringValue(String),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // equality is only defined between the same variant — a byte count of 1 is not equal to an integer 1
            (Self::BooleanValue(a), Self::BooleanValue(b)) => a == b,
            (Self::BytesValue(a), Self::BytesValue(b)) => a == b,
            (Self::IntegerValue(a), Self::IntegerValue(b)) => a == b,
            (Self::StringValue(a), Self::StringValue(b)) => a == b,
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
            Self::StringValue(value) => write!(f, "{value}"),
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

impl Value {
    pub fn from_bytes(value: &str) -> Result<Self, ValueParseError> {
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
                return Ok(Self::BytesValue(bytes));
            }
        }
        Err(ValueParseError::InvalidBytes(format!(
            "Unknown format: {}",
            value
        )))
    }

    pub fn from_integer(value: &str) -> Result<Self, ValueParseError> {
        let value = value.trim();
        // Count units are decimal multipliers (K = 1_000, M = 1_000_000, G = 1_000_000_000),
        // distinct from the binary (1024-based) size units handled by `from_bytes`. They are
        // matched case-insensitively to mirror the grammar's `count_unit` rule. A bare number
        // (no suffix) falls through to the plain parse below.
        let upper = value.to_uppercase();
        let units = [
            ("G", 1_000_000_000_u64),
            ("M", 1_000_000_u64),
            ("K", 1_000_u64),
        ];
        for (suffix, multiplier) in units {
            if let Some(number) = upper.strip_suffix(suffix) {
                let integer = number
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| ValueParseError::InvalidInteger(value.to_string()))?;
                let scaled = integer
                    .checked_mul(multiplier)
                    .ok_or_else(|| ValueParseError::InvalidInteger(value.to_string()))?;
                return Ok(Self::IntegerValue(scaled));
            }
        }
        value
            .parse::<u64>()
            .map(Self::IntegerValue)
            .map_err(|_| ValueParseError::InvalidInteger(value.to_string()))
    }

    pub fn from_boolean(value: &str) -> Result<Self, ValueParseError> {
        value
            .trim()
            .parse::<bool>()
            .map(Self::BooleanValue)
            .map_err(|_| ValueParseError::InvalidBoolean(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_parses_bytes() {
        assert_eq!(Value::from_bytes("100B").unwrap(), Value::BytesValue(100));
    }

    #[test]
    fn from_bytes_parses_kilobytes() {
        assert_eq!(Value::from_bytes("1KB").unwrap(), Value::BytesValue(1024));
    }

    #[test]
    fn from_bytes_parses_megabytes() {
        assert_eq!(
            Value::from_bytes("1MB").unwrap(),
            Value::BytesValue(1024 * 1024)
        );
    }

    #[test]
    fn from_bytes_parses_gigabytes() {
        assert_eq!(
            Value::from_bytes("1GB").unwrap(),
            Value::BytesValue(1024 * 1024 * 1024)
        );
    }

    #[test]
    fn from_bytes_is_case_insensitive() {
        assert_eq!(Value::from_bytes("1kb").unwrap(), Value::BytesValue(1024));
    }

    #[test]
    fn from_bytes_trims_whitespace() {
        assert_eq!(Value::from_bytes(" 1KB ").unwrap(), Value::BytesValue(1024));
    }

    #[test]
    fn from_bytes_rejects_bare_number() {
        assert!(matches!(
            Value::from_bytes("1024"),
            Err(ValueParseError::InvalidBytes(_))
        ));
    }

    #[test]
    fn from_bytes_rejects_non_numeric_prefix() {
        assert!(matches!(
            Value::from_bytes("abcKB"),
            Err(ValueParseError::InvalidBytes(_))
        ));
    }

    #[test]
    fn from_integer_parses_number() {
        assert_eq!(Value::from_integer("42").unwrap(), Value::IntegerValue(42));
    }

    #[test]
    fn from_integer_parses_zero() {
        assert_eq!(Value::from_integer("0").unwrap(), Value::IntegerValue(0));
    }

    #[test]
    fn from_integer_trims_whitespace() {
        assert_eq!(
            Value::from_integer(" 42 ").unwrap(),
            Value::IntegerValue(42)
        );
    }

    #[test]
    fn from_integer_parses_count_units() {
        assert_eq!(
            Value::from_integer("5K").unwrap(),
            Value::IntegerValue(5_000)
        );
        assert_eq!(
            Value::from_integer("5M").unwrap(),
            Value::IntegerValue(5_000_000)
        );
        assert_eq!(
            Value::from_integer("5G").unwrap(),
            Value::IntegerValue(5_000_000_000)
        );
    }

    #[test]
    fn from_integer_count_units_are_case_insensitive() {
        assert_eq!(
            Value::from_integer("5k").unwrap(),
            Value::IntegerValue(5_000)
        );
        assert_eq!(
            Value::from_integer("5m").unwrap(),
            Value::IntegerValue(5_000_000)
        );
        assert_eq!(
            Value::from_integer("5g").unwrap(),
            Value::IntegerValue(5_000_000_000)
        );
    }

    #[test]
    fn from_integer_count_unit_without_number_is_rejected() {
        assert!(matches!(
            Value::from_integer("K"),
            Err(ValueParseError::InvalidInteger(_))
        ));
    }

    #[test]
    fn from_integer_rejects_size_unit() {
        // KB/MB/GB are binary size units, not count units, so they do not parse as integers
        assert!(matches!(
            Value::from_integer("5KB"),
            Err(ValueParseError::InvalidInteger(_))
        ));
    }

    #[test]
    fn from_integer_rejects_text() {
        assert!(matches!(
            Value::from_integer("abc"),
            Err(ValueParseError::InvalidInteger(_))
        ));
    }

    #[test]
    fn from_integer_rejects_float() {
        assert!(matches!(
            Value::from_integer("1.5"),
            Err(ValueParseError::InvalidInteger(_))
        ));
    }

    #[test]
    fn from_boolean_parses_true() {
        assert_eq!(
            Value::from_boolean("true").unwrap(),
            Value::BooleanValue(true)
        );
    }

    #[test]
    fn from_boolean_parses_false() {
        assert_eq!(
            Value::from_boolean("false").unwrap(),
            Value::BooleanValue(false)
        );
    }

    #[test]
    fn from_boolean_trims_whitespace() {
        assert_eq!(
            Value::from_boolean(" true ").unwrap(),
            Value::BooleanValue(true)
        );
    }

    #[test]
    fn from_boolean_rejects_yes() {
        assert!(matches!(
            Value::from_boolean("yes"),
            Err(ValueParseError::InvalidBoolean(_))
        ));
    }

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
        assert_eq!(
            Value::BooleanValue(true).partial_cmp(&Value::BooleanValue(false)),
            None
        );
    }

    #[test]
    fn ord_cross_variant_returns_none() {
        assert_eq!(
            Value::BytesValue(1).partial_cmp(&Value::IntegerValue(1)),
            None
        );
    }
}
