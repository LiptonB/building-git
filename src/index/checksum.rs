use std::io::{Read, Result, Write};
use std::iter;

use crypto::digest::Digest;

pub struct ChecksummedFile<I, D: Digest> {
    inner: I,
    hasher: D,
}

impl<I, D: Digest> ChecksummedFile<I, D> {
    pub fn new(inner: I, hasher: D) -> Self {
        Self { inner, hasher }
    }
}

impl<I: Write, D: Digest> ChecksummedFile<I, D> {
    pub fn write_hash(&mut self) -> Result<usize> {
        let mut hash: Vec<u8> = iter::repeat(0)
            .take((self.hasher.output_bits() + 7) / 8)
            .collect();
        self.hasher.result(&mut hash);
        self.inner.write(&hash)
    }
}

impl<I: Write, D: Digest> Write for ChecksummedFile<I, D> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.hasher.input(buf);
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<I: Read, D: Digest> Read for ChecksummedFile<I, D> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let out = self.inner.read(buf)?;
        self.hasher.input(buf);
        Ok(out)
    }
}
