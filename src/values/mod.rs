mod errors;
mod value;
pub use errors::ValueParseError;
pub use value::Value::BytesValue;
pub use value::{parse_boolean, parse_bytes, parse_integer, Value};
