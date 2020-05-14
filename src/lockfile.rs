use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::PathBuf;

use anyhow::{bail, Result};

pub struct Lockfile {
    path: PathBuf,
    lock_path: PathBuf,
}

pub struct LockfileGuard<'a> {
    file: Option<File>,
    lockfile: &'a Lockfile,
}

impl Lockfile {
    pub fn new(file: PathBuf) -> Self {
        let lock_path = file.with_extension("lock");
        Self {
            path: file,
            lock_path,
        }
    }

    pub fn hold_for_update(&self) -> Result<Option<LockfileGuard>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&self.lock_path);

        let file = match file {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => return Ok(None),
            Err(err) => bail!(err),
        };

        Ok(Some(LockfileGuard {
            file: Some(file),
            lockfile: self,
        }))
    }
}

impl LockfileGuard<'_> {
    pub fn commit(self) -> Result<()> {
        fs::rename(&self.lockfile.lock_path, &self.lockfile.path)?;
        Ok(())
    }
}

impl Write for LockfileGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file
            .as_ref()
            .expect("File unexpectedly None")
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_ref().expect("File unexpectedly None").flush()
    }
}
