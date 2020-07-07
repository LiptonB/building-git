use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::PathBuf;

use anyhow::{bail, Result};

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
