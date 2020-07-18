mod checksum;

use std::cmp;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fs::{File, Metadata};
use std::io::{Read, Write};
use std::iter;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use cookie_factory as cf;
use crypto::sha1::Sha1;

use self::checksum::*;
use crate::lockfile::*;
use crate::workspace::*;

pub struct Index {
    entries: BTreeMap<PathBuf, Entry>,
    file: Option<ChecksummedFile<Lockfile, Sha1>>,
    changed: bool,
}

#[derive(Debug)]
pub struct Entry {
    pub ctime: u32,
    pub ctime_nsec: u32,
    pub mtime: u32,
    pub mtime_nsec: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub oid: Vec<u8>,
    pub flags: u16,
    pub path: String,
}

impl Index {
    const HEADER_SIZE: usize = 12;
    const SIGNATURE: &'static [u8] = b"DIRC";
    const VERSION: u32 = 2;

    pub fn load_for_update(path: PathBuf) -> Result<Self> {
        let lockfile =
            Lockfile::hold_for_update(path.clone())?.ok_or(anyhow!("Index file is locked"))?;

        let mut index = Self::load(path)?;
        index.file = Some(ChecksummedFile::new(lockfile, Sha1::new()));

        Ok(index)
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        let entries = match File::open(&path) {
            Ok(indexfile) => Self::load_entries(&indexfile)?,
            Err(_) => BTreeMap::new(),
        };

        Ok(Self {
            entries,
            file: None,
            changed: false,
        })
    }

    pub fn add(&mut self, file: &WorkspacePath, oid: &str, metadata: &Metadata) {
        let entry = Entry::new(file, oid, metadata);
        self.entries.insert(file.rel_path().to_owned(), entry);
        self.changed = true;
    }

    fn serialize_entries<'a, W: Write + 'a>(
        entries: &'a BTreeMap<PathBuf, Entry>,
    ) -> impl cf::SerializeFn<W> + 'a {
        use cf::{bytes::be_u32, combinator::slice, multi::all, sequence::tuple};

        tuple((
            slice(Self::SIGNATURE),
            be_u32(Self::VERSION),
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

    fn read_header<R: Read>(mut indexfile: R) -> Result<usize> {
        use nom::{bytes::streaming::take, number::streaming::be_u32, sequence::tuple, IResult};

        fn parse_header<'a>(input: &'a [u8]) -> IResult<&'a [u8], (&'a [u8], u32, u32)> {
            tuple((
                take(Index::SIGNATURE.len()), // signature
                be_u32,                       // version
                be_u32,                       // count
            ))(input)
        }

        let mut data = [0; Self::HEADER_SIZE];
        indexfile.read(&mut data)?;

        let (extra, (signature, version, count)) =
            parse_header(&data).map_err(|e| anyhow!("{:?}", e))?;
        if signature != Self::SIGNATURE {
            bail!(
                "Signature: expected '{:?}' but found '{:?}'",
                Self::SIGNATURE,
                signature
            );
        }
        if version != Self::VERSION {
            bail!(
                "Version: expected '{}' but found '{}'",
                Self::VERSION,
                version
            );
        }
        if !extra.is_empty() {
            bail!("Programmer error: Unexpected extra data: {:?}", extra);
        }

        Ok(count as usize)
    }

    fn read_entries<R: Read>(mut indexfile: R, count: usize) -> Result<BTreeMap<PathBuf, Entry>> {
        use nom::{
            bytes::complete::tag,
            bytes::streaming::{take, take_until},
            multi::many_m_n,
            number::streaming::{be_u16, be_u32},
            sequence::{terminated, tuple},
            Err, IResult,
        };

        fn parse_entry(
            input: &[u8],
        ) -> IResult<
            &[u8],
            (
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                u32,
                &[u8],
                u16,
                &[u8],
            ),
        > {
            terminated(
                tuple((
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    be_u32,
                    take(20u8),
                    be_u16,
                    take_until("\0"),
                )),
                many_m_n(1, 8, tag("\0")),
            )(input)
        }

        let mut entries: BTreeMap<PathBuf, Entry> = BTreeMap::new();
        let mut data = vec![0; Entry::ENTRY_MIN_SIZE];
        indexfile.read(&mut data)?;

        while entries.len() < count {
            match parse_entry(&data) {
                Ok((extra, entrydata)) => {
                    if !extra.is_empty() {
                        bail!("Programmer error: Unexpected extra data: {:?}", extra);
                    }

                    let entry = Entry::load(entrydata);
                    let path = PathBuf::from(&entry.path);
                    entries.insert(path, entry);

                    let amount = indexfile.read(&mut data[..Entry::ENTRY_MIN_SIZE])?;
                    if amount == 0 {
                        bail!("Unexpected EOF");
                    }
                    data.resize(amount, 0);
                }
                Err(Err::Incomplete(_)) => {
                    let current_len = data.len();
                    data.resize(current_len + Entry::ENTRY_BLOCK, 0);
                    let amount = indexfile.read(&mut data[current_len..])?;
                    if amount == 0 {
                        bail!("Unexpected EOF");
                    }
                }
                Err(_) => bail!("Index parse error"),
            }
        }

        Ok(entries)
    }

    pub fn write_updates(mut self) -> Result<()> {
        if !self.changed {
            // Lockfile will be deleted when self is dropped
            return Ok(());
        }

        let mut file = self
            .file
            .take()
            .expect("Programmer error: index was not locked for writing");

        cf::gen_simple(Self::serialize_entries(&self.entries), &mut file)?;

        file.write_hash()?;
        file.into_inner().commit()?;

        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }
}

impl Entry {
    const REGULAR_MODE: u32 = 0o100644;
    const EXECUTABLE_MODE: u32 = 0o100755;
    const MAX_PATH_SIZE: usize = 0xfff;
    const ENTRY_BLOCK: usize = 8;
    const ENTRY_MIN_SIZE: usize = 64;

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

    fn load(
        loaded_data: (
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            &[u8],
            u16,
            &[u8],
        ),
    ) -> Self {
        let (
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid,
            flags,
            path,
        ) = loaded_data;

        Self {
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            oid: oid.to_vec(),
            flags,
            path: String::from_utf8_lossy(path).into_owned(),
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
