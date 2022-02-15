use std::io::{Error as IOError, Read, Write};
use std::iter;

use anyhow::{Context, Error};
use crypto::digest::Digest;

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
        tracing::debug!(bytes = read.len(), "About to read from checksummed file");
        self.inner
            .read_exact(&mut read)
            .context("Reading checksum")?;

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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::PathBuf;

    use crypto::sha1::Sha1;
    use tempfile::{tempdir, TempDir};

    use super::ChecksummedFile;

    struct Fixture {
        tempdir: TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let tempdir = tempdir().expect("tempdir");

            Self { tempdir }
        }

        fn get_filename(&self) -> PathBuf {
            let mut filename = PathBuf::from(&self.tempdir.path());
            filename.push("testfile");

            filename
        }

        fn create_file(&self) -> ChecksummedFile<File, Sha1> {
            let filename = self.get_filename();
            let file = File::create(&filename).expect("File::create");
            ChecksummedFile::new(file, Sha1::new())
        }

        fn open_file(&self) -> ChecksummedFile<File, Sha1> {
            let filename = self.get_filename();
            let file = File::open(&filename).expect("File::open");
            ChecksummedFile::new(file, Sha1::new())
        }
    }

    #[test]
    fn can_write_checksummed_file() {
        let fixture = Fixture::new();

        {
            let mut file = fixture.create_file();
            file.write_all(b"test_contents").expect("write_all");
            file.write_hash().expect("write_hash");
        }

        let expected = b"test_contents\x57\xc5\x84\x76\x41\xe1\xac\xef\xc8\xf9\xeb\xe8\x1d\x21\x13\x0b\xfa\x0c\x75\x54";
        let actual = std::fs::read(&fixture.get_filename()).expect("read");
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_read_checksummed_file() {
        let fixture = Fixture::new();

        {
            let mut file = fixture.create_file();
            file.write_all(b"test_contents").expect("write_all");
            file.write_hash().expect("write_hash");
        }

        let mut file = fixture.open_file();
        let mut data: [u8; 13] = [0; 13];
        file.read_exact(&mut data).expect("read_exact");
        assert_eq!(&data, b"test_contents");
        assert!(file.verify_checksum().expect("verify_checksum"));
    }
}
