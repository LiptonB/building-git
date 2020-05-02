use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

pub struct Refs {
    root: PathBuf,
}

impl Refs {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root: path.as_ref().to_owned(),
        }
    }

    pub fn update_head(&self, oid: &str) -> Result<()> {
        fs::write(&self.head_path(), oid)?;
        Ok(())
    }

    pub fn read_head(&self) -> Result<Option<String>> {
        let result = fs::read(&self.head_path());
        match result {
            Ok(data) => Ok(Some(String::from_utf8_lossy(&data).into_owned())),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => bail!(err),
        }
    }

    fn head_path(&self) -> PathBuf {
        self.root.join("HEAD")
    }
}
