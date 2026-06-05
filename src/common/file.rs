use anyhow::{bail, Result};
use std::path::Path;

pub fn assert_file_exists(file: &Path) -> Result<()> {
    if !file.exists() {
        bail!("File {} does not exist", file.display());
    }

    Ok(())
}
