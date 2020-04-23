use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::path::{Path, PathBuf};

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

        const MODE: &[u8] = b"100644";
        self.entries
            .iter()
            .map(|entry| {
                let oid = entry
                    .oid
                    .from_hex()
                    .expect("Hash is not a valid hex string");
                let path = entry.rel_path.to_string_lossy().as_bytes().to_owned();
                let parts = vec![MODE.to_owned(), b" ".to_vec(), path, b"\0".to_vec(), oid];
                parts
            }) // Iterator<Vec<Vec<u8>>>
            .flatten() // Iterator<Vec<u8>>
            .flatten() // Iterator<u8>
            .collect() // Vec<u8>
    }
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    rel_path: PathBuf,
    oid: String,
}

impl TreeEntry {
    pub fn new<P: AsRef<Path>>(rel_path: P, oid: &str) -> Self {
        Self {
            rel_path: rel_path.as_ref().to_owned(),
            oid: oid.to_owned(),
        }
    }
}
