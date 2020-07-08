mod checksum;

use std::cmp;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fs::{File, Metadata};
use std::io::{Read, Write};
use std::iter;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use cookie_factory as cf;
use crypto::sha1::Sha1;

use self::checksum::*;
use crate::lockfile::*;
use crate::workspace::*;

pub struct Index {
    entries: BTreeMap<PathBuf, Entry>,
    file: ChecksummedFile<Lockfile, Sha1>,
}

struct Entry {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    oid: Vec<u8>,
    flags: u16,
    path: String,
}

impl Index {
    pub fn load_for_update(path: PathBuf) -> Result<Self> {
        let lockfile =
            Lockfile::hold_for_update(path.clone())?.ok_or(anyhow!("Index file is locked"))?;

        let mut indexfile = File::open(&path)?;
        let entries = Self::load_entries(&indexfile)?;

        Ok(Self {
            entries,
            file: ChecksummedFile::new(lockfile, Sha1::new()),
        })
    }

    pub fn add(&mut self, file: &WorkspacePath, oid: &str, metadata: &Metadata) {
        let entry = Entry::new(file, oid, metadata);
        self.entries.insert(file.rel_path().to_owned(), entry);
    }

    fn serialize_entries<'a, W: Write + 'a>(
        entries: &'a BTreeMap<PathBuf, Entry>,
    ) -> impl cf::SerializeFn<W> + 'a {
        use cf::{bytes::be_u32, combinator::slice, multi::all, sequence::tuple};

        tuple((
            slice(b"DIRC"),
            be_u32(2),
            be_u32(entries.len().try_into().unwrap()),
            all(entries.values().map(Entry::serialize)),
        ))
    }

    fn load_entries<R: Read>(indexfile: R) -> Result<BTreeMap<PathBuf, Entry>> {
        let mut indexfile = ChecksummedFile::new(indexfile, Sha1::new());

        let count = Self::read_header(&mut indexfile)?;
        let entries = Self::read_entries(&mut indexfile, count)?;

        indexfile.verify_checksum()?;

        Ok(entries)
    }

    fn read_header<R: Read>(indexfile: R) -> Result<usize> {
        unimplemented!();
    }

    fn read_entries<R: Read>(indexfile: R, count: usize) -> Result<BTreeMap<PathBuf, Entry>> {
        let mut entries = BTreeMap::new();

        unimplemented!();

        Ok(entries)
    }

    pub fn write_updates(mut self) -> Result<()> {
        cf::gen_simple(Self::serialize_entries(&self.entries), &mut self.file)?;

        self.file.write_hash()?;
        self.file.into_inner().commit()?;

        Ok(())
    }
}

impl Entry {
    const REGULAR_MODE: u32 = 0o100644;
    const EXECUTABLE_MODE: u32 = 0o100755;
    const MAX_PATH_SIZE: usize = 0xfff;
    const ENTRY_BLOCK: usize = 8;

    fn new(file: &WorkspacePath, oid: &str, metadata: &Metadata) -> Self {
        use rustc_serialize::hex::FromHex;
        use std::os::unix::fs::MetadataExt;

        let mode = if metadata.mode() & 0o100 == 0 {
            Entry::REGULAR_MODE
        } else {
            Entry::EXECUTABLE_MODE
        };
        let path = file.rel_path().to_string_lossy().into_owned();
        let flags = cmp::min(path.len(), Entry::MAX_PATH_SIZE) as u16;

        Self {
            ctime: metadata.ctime().try_into().unwrap(),
            ctime_nsec: metadata.ctime_nsec().try_into().unwrap(),
            mtime: metadata.mtime().try_into().unwrap(),
            mtime_nsec: metadata.mtime_nsec().try_into().unwrap(),
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode,
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size() as u32,
            oid: oid.from_hex().expect("oid is not a valid hex string"),
            flags,
            path,
        }
    }

    fn serialize<'a, W: Write + 'a>(&'a self) -> impl cf::SerializeFn<W> + 'a {
        use cf::{
            bytes::{be_u16, be_u32},
            combinator::{slice, string},
            sequence::tuple,
        };

        align(
            Entry::ENTRY_BLOCK,
            tuple((
                be_u32(self.ctime),
                be_u32(self.ctime_nsec),
                be_u32(self.mtime),
                be_u32(self.mtime_nsec),
                be_u32(self.dev),
                be_u32(self.ino),
                be_u32(self.mode), // 00 01 89 24 (should be 00 00 81 a4)
                be_u32(self.uid),
                be_u32(self.gid),
                be_u32(self.size),
                slice(&self.oid),
                be_u16(self.flags),
                string(&self.path),
                slice(b"\0"),
            )),
        )
    }
}

fn align<W: Write, F>(amount: usize, f: F) -> impl cf::SerializeFn<W>
where
    F: cf::SerializeFn<W>,
{
    use cf::{bytes::be_u8, multi::all};

    move |out: cf::WriteContext<W>| {
        let start: usize = out.position.try_into().unwrap();
        let out = f(out)?;
        let end: usize = out.position.try_into().unwrap();
        let missing = (amount - ((end - start) % amount)) % amount;
        all(iter::repeat(b'\0').take(missing).map(be_u8))(out)
    }
}
