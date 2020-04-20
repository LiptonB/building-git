mod objects;

use anyhow::{bail, Result};
use flate2::{write::ZlibEncoder, Compression};
pub use objects::*;
use rand::prelude::*;
use std::fs::{create_dir_all, rename, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

pub struct Database {
    root: PathBuf,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root: path.as_ref().to_owned(),
        }
    }

    // TODO: Can this be more generic?
    pub fn store(&self, object: BoxObject) -> Result<()> {
        let object_type = object.object_type();
        let object_bytes = object.to_bytes();
        let len_tag = format!("{}", object_bytes.len());

        let mut serialized =
            Vec::with_capacity(object_type.len() + object_bytes.len() + len_tag.len() + 2);
        serialized.extend_from_slice(object_type.as_ref());
        serialized.push(b' ');
        serialized.extend_from_slice(len_tag.as_ref());
        serialized.push(b'\0');
        serialized.extend_from_slice(&object_bytes);

        let oid = object.oid();
        self.write_object(&oid, &serialized)?;
    }

    fn write_object(&self, oid: &str, content: &[u8]) -> Result<()> {
        let object_path = self
            .root
            .join(Path::new(&oid[0..2]))
            .join(Path::new(&oid[2..]));
        let dir = object_path.parent().expect("Path error");
        let (tempfile, tempfile_name) = self.open_tempfile(&dir)?;
        let mut encoder = ZlibEncoder::new(&tempfile, Compression::fast());
        encoder.write_all(content)?;
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
