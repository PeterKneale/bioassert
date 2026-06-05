use anyhow::{bail, Result};

pub fn parse_int(s: &str) -> Result<u64> {
    match s.parse::<u64>() {
        Ok(value) => Ok(value),
        Err(_) => bail!("Could not parse integer from '{}'", s),
    }
}
