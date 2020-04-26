mod blob;
mod commit;
mod object;
mod tree;

use std::fs::{create_dir_all, rename, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use flate2::{write::ZlibEncoder, Compression};
use rand::prelude::*;

pub use blob::*;
pub use commit::*;
pub use object::*;
pub use tree::*;

pub struct Database {
    root: PathBuf,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root: path.as_ref().to_owned(),
        }
    }

    pub fn store<O: Object>(&self, object: &mut O) -> Result<()> {
        compute_oid(object);
        let oid = object.oid();
        let content = to_bytes(object);

        let object_path = self
            .root
            .join(Path::new(&oid[0..2]))
            .join(Path::new(&oid[2..]));
        let dir = object_path.parent().expect("Path error");
        let (tempfile, tempfile_name) = self.open_tempfile(&dir)?;
        let mut encoder = ZlibEncoder::new(&tempfile, Compression::fast());
        encoder.write_all(&content)?;
        rename(tempfile_name, object_path)?;

        Ok(())
    }

    fn open_tempfile<P: AsRef<Path>>(&self, dir: P) -> Result<(File, PathBuf)> {
        let chars = (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'0'..=b'9');
        let mut rng = thread_rng();
        let random_part = chars.choose_multiple(&mut rng, 6);

        let name = format!("tmp_obj_{}", String::from_utf8_lossy(&random_part));
        let path = dir.as_ref().join(name);
        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    create_dir_all(dir)?;
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)?
                } else {
                    bail!(err)
                }
            }
        };

        Ok((file, path))
    }
}
