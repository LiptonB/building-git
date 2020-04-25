use std::fmt;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crypto::{digest::Digest, sha1::Sha1};
use time::OffsetDateTime;

pub trait Object {
    fn object_type(&self) -> &str;
    fn oid(&self) -> String;
    fn to_bytes(&self) -> Vec<u8>;
}

#[derive(Debug, Clone)]
pub struct Blob {
    data: Vec<u8>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Object for Blob {
    fn object_type(&self) -> &str {
        "blob"
    }

    fn oid(&self) -> String {
        let mut hasher = Sha1::new();
        hasher.input(&self.data);
        hasher.result_str()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Tree {
    entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new(entries: &[TreeEntry]) -> Self {
        let mut entries = entries.to_vec();
        entries.sort_unstable_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Self { entries }
    }
}

impl Object for Tree {
    fn object_type(&self) -> &str {
        "tree"
    }

    fn oid(&self) -> String {
        let mut hasher = Sha1::new();
        // TODO: caching?
        hasher.input(&self.to_bytes());
        hasher.result_str()
    }

    fn to_bytes(&self) -> Vec<u8> {
        use rustc_serialize::hex::FromHex;

        self.entries
            .iter()
            .map(|entry| {
                let oid = entry
                    .oid
                    .from_hex()
                    .expect("Hash is not a valid hex string");
                let path = entry.rel_path.to_string_lossy().as_bytes().to_owned();
                let mode = entry.mode().as_bytes().to_owned();
                let parts = vec![mode, b" ".to_vec(), path, b"\0".to_vec(), oid];
                parts
            }) // Iterator<Vec<Vec<u8>>>
            .flatten() // Iterator<Vec<u8>>
            .flatten() // Iterator<u8>
            .collect() // Vec<u8>
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    tree: String,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(tree: &str, author: Author, message: &str) -> Self {
        Self {
            tree: tree.to_owned(),
            author,
            message: message.to_owned(),
        }
    }
}

impl Object for Commit {
    fn object_type(&self) -> &str {
        "commit"
    }

    fn oid(&self) -> String {
        let mut hasher = Sha1::new();
        // TODO: caching?
        hasher.input(&self.to_bytes());
        hasher.result_str()
    }

    fn to_bytes(&self) -> Vec<u8> {
        format!(
            "tree {}
author {}
committer {}

{}",
            self.tree, self.author, self.author, self.message
        )
        .as_bytes()
        .to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    rel_path: PathBuf,
    oid: String,
    mode: u32,
}

impl TreeEntry {
    const REGULAR_MODE: &'static str = "100644";
    const EXECUTABLE_MODE: &'static str = "100755";

    pub fn new<P: AsRef<Path>>(rel_path: P, oid: &str, metadata: &Metadata) -> Self {
        Self {
            rel_path: rel_path.as_ref().to_owned(),
            oid: oid.to_owned(),
            mode: metadata.mode(),
        }
    }

    pub fn mode(&self) -> &str {
        let is_executable = self.mode & 0o100 != 0;

        if is_executable {
            Self::EXECUTABLE_MODE
        } else {
            Self::REGULAR_MODE
        }
    }
}

#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    timestamp: OffsetDateTime,
}

impl Author {
    pub fn new(name: &str, email: &str, timestamp: OffsetDateTime) -> Self {
        Self {
            name: name.to_owned(),
            email: email.to_owned(),
            timestamp,
        }
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} <{}> {} {}",
            self.name,
            self.email,
            self.timestamp.timestamp(),
            self.timestamp.offset().format("%z")
        )
    }
}
