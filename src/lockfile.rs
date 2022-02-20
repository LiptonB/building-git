use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::mem;
use std::path::PathBuf;

use anyhow::{bail, Result};

#[derive(Debug)]
pub struct Lockfile {
    path: PathBuf,
    lock_path: PathBuf,
    file: File,
}

impl Lockfile {
    pub fn hold_for_update(path: PathBuf) -> Result<Option<Self>> {
        let lock_path = path.with_extension("lock");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&lock_path);

        let file = match file {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => return Ok(None),
            Err(err) => bail!(err),
        };

        Ok(Some(Self {
            path,
            lock_path,
            file,
        }))
    }

    pub fn commit(self) -> Result<()> {
        fs::rename(&self.lock_path, &self.path)?;
        mem::forget(self);
        Ok(())
    }
}

impl Write for Lockfile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Drop for Lockfile {
    fn drop(&mut self) {
        fs::remove_file(&self.lock_path)
            .unwrap_or_else(|_| panic!("Failed to remove lock file: {}", self.lock_path.display()));
    }
}
