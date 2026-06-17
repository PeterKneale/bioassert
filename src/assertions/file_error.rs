use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileError {
    pub path: PathBuf,
    pub source: std::io::Error,
}

impl FileError {
    pub fn new(path: &Path, source: std::io::Error) -> Self {
        Self { path: path.to_path_buf(), source }
    }
}

impl Error for FileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl Display for FileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.source)
    }
}
