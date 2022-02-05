use std::io::{Read, Error as IOError, Write};
use std::iter;

use crypto::digest::Digest;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error")]
    Io {
        #[from]
        source: std::io::Error,
    }
}

pub struct ChecksummedFile<I, D: Digest> {
    inner: I,
    hasher: D,
}

impl<I, D: Digest> ChecksummedFile<I, D> {
    pub fn new(inner: I, hasher: D) -> Self {
        Self { inner, hasher }
    }

    pub fn into_inner(self) -> I {
        self.inner
    }

    pub fn hash(&mut self) -> Vec<u8> {
        let mut hash: Vec<u8> = iter::repeat(0)
            .take((self.hasher.output_bits() + 7) / 8)
            .collect();
        self.hasher.result(&mut hash);
        hash
    }
}

impl<I: Write, D: Digest> ChecksummedFile<I, D> {
    pub fn write_hash(&mut self) -> Result<usize, Error> {
        let hash = self.hash();
        Ok(self.inner.write(&hash)?)
    }
}

impl<I: Read, D: Digest> ChecksummedFile<I, D> {
    pub fn verify_checksum(&mut self) -> Result<bool, Error> {
        let computed = self.hash();
        let mut read: Vec<u8> = iter::repeat(0).take(computed.len()).collect();
        self.inner.read_exact(&mut read)?;

        Ok(computed == read)
    }
}

impl<I: Write, D: Digest> Write for ChecksummedFile<I, D> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IOError> {
        self.hasher.input(buf);
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<(), IOError> {
        self.inner.flush()
    }
}

impl<I: Read, D: Digest> Read for ChecksummedFile<I, D> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IOError> {
        let out = self.inner.read(buf)?;
        self.hasher.input(buf);
        Ok(out)
    }
}
