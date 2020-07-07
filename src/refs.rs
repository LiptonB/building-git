use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::lockfile::*;

pub struct Refs {
    root: PathBuf,
}

impl Refs {
    pub fn new(path: PathBuf) -> Self {
        Self { root: path }
    }

    pub fn update_head(&self, oid: &str) -> Result<()> {
        if let Some(mut head) = Lockfile::hold_for_update(self.head_path())? {
            head.write(oid.as_bytes())?;
            head.write(b"\n")?;
            head.commit()?;
        } else {
            bail!(
                "Could not acquire lock on file: {}",
                self.head_path().display()
            );
        }
        Ok(())
    }

    pub fn read_head(&self) -> Result<Option<String>> {
        let result = fs::read(&self.head_path());
        match result {
            Ok(data) => Ok(Some(String::from_utf8_lossy(&data).trim().to_owned())),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
            Err(err) => bail!(err),
        }
    }

    fn head_path(&self) -> PathBuf {
        self.root.join("HEAD")
    }
}
