use std::collections::HashMap;
use std::fmt;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use crypto::{digest::Digest, sha1::Sha1};
use time::OffsetDateTime;

// TODO: would an enum make more sense since it seems like content is the only real function
// needing to be overloaded?
pub trait Object {
    fn object_type(&self) -> &str;
    fn content(&self) -> Vec<u8>;

    fn to_bytes(&self) -> Vec<u8> {
        let object_type = self.object_type();
        let content = self.content();
        let len_tag = content.len().to_string();

        let mut serialized =
            Vec::with_capacity(object_type.len() + len_tag.len() + content.len() + 2);
        serialized.extend_from_slice(object_type.as_ref());
        serialized.push(b' ');
        serialized.extend_from_slice(len_tag.as_ref());
        serialized.push(b'\0');
        serialized.extend_from_slice(&content);

        serialized
    }

    fn oid(&self) -> String {
        let mut hasher = Sha1::new();
        // TODO: caching?
        hasher.input(&self.to_bytes());
        hasher.result_str()
    }
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

    fn content(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[derive(Debug, Clone)]
enum TreeEntry {
    Tree(Tree),
    File(TreeFile),
}

#[derive(Debug, Clone)]
pub struct Tree {
    entries: HashMap<String, TreeEntry>,
    key_order: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TreeFile {
    rel_path: PathBuf,
    oid: String,
    mode: u32,
}

impl TreeEntry {
    const DIRECTORY_MODE: &'static str = "40000";

    fn oid(&self) -> String {
        match self {
            TreeEntry::Tree(tree) => tree.oid(),
            TreeEntry::File(file) => file.oid.clone(),
        }
    }

    fn mode(&self) -> &str {
        match self {
            TreeEntry::Tree(_) => Self::DIRECTORY_MODE,
            TreeEntry::File(file) => file.mode(),
        }
    }
}

impl Tree {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            key_order: Vec::new(),
        }
    }

    pub fn build(mut entries: Vec<TreeFile>) -> Result<Self> {
        entries.sort_unstable_by(|a, b| {
            a.rel_path
                .to_string_lossy()
                .cmp(&b.rel_path.to_string_lossy())
        });

        let mut root = Self::new();
        for entry in entries {
            let ancestors = entry.ancestors();
            println!(
                "Ancestors of {} are {:?}",
                entry.rel_path.display(),
                ancestors
            );
            root.add_entry(&ancestors, entry)?;
        }

        Ok(root)
    }

    fn add_entry<P: AsRef<Path>>(&mut self, parents: &[P], entry: TreeFile) -> Result<()> {
        if parents.is_empty() {
            let name = entry
                .rel_path
                .file_name()
                .ok_or(anyhow!("Missing filename in {:?}", entry))?
                .to_string_lossy()
                .into_owned();
            self.key_order.push(name.clone());
            self.entries.insert(name, TreeEntry::File(entry));
        } else {
            let first_parent = {
                parents[0]
                    .as_ref()
                    .file_name()
                    .ok_or(anyhow!(
                        "Missing filename in {:?}",
                        parents[parents.len() - 1].as_ref()
                    ))?
                    .to_string_lossy()
                    .into_owned()
            };
            if !self.entries.contains_key(&first_parent) {
                self.key_order.push(first_parent.clone());
                self.entries
                    .insert(first_parent.clone(), TreeEntry::Tree(Tree::new()));
            }
            if let TreeEntry::Tree(ref mut tree) = self.entries.get_mut(&first_parent).unwrap() {
                tree.add_entry(&parents[1..], entry)?;
            } else {
                return Err(anyhow!(
                    "A parent of {} is not a directory",
                    entry.rel_path.display()
                ));
            }
        }
        Ok(())
    }

    pub fn traverse(&self, callback: &dyn Fn(&Tree) -> Result<()>) -> Result<()> {
        for key in &self.key_order {
            if let TreeEntry::Tree(ref tree) = self.entries[key] {
                tree.traverse(callback)?;
            }
        }
        callback(self)?;
        Ok(())
    }
}

impl Object for Tree {
    fn object_type(&self) -> &str {
        "tree"
    }

    fn content(&self) -> Vec<u8> {
        use rustc_serialize::hex::FromHex;

        self.key_order
            .iter()
            .map(|key| {
                let entry = &self.entries[key];
                let oid = entry
                    .oid()
                    .from_hex()
                    .expect("Hash is not a valid hex string");
                let mode = entry.mode().as_bytes().to_owned();
                let parts = vec![
                    mode,
                    b" ".to_vec(),
                    key.as_bytes().to_vec(),
                    b"\0".to_vec(),
                    oid,
                ];
                parts
            }) // Iterator<Vec<Vec<u8>>>
            .flatten() // Iterator<Vec<u8>>
            .flatten() // Iterator<u8>
            .collect() // Vec<u8>
    }
}

impl TreeFile {
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

    pub fn ancestors(&self) -> Vec<String> {
        use std::path::Component::*;
        let mut ancestors = Vec::new();
        let components = self
            .rel_path
            .parent()
            .expect("Unexpected absolute path")
            .components();
        for component in components {
            match component {
                Normal(s) => ancestors.push(s.to_string_lossy().into_owned()),
                _ => panic!("Not properly canonicalized"),
            }
        }
        ancestors
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

    fn content(&self) -> Vec<u8> {
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
            self.timestamp.offset().lazy_format("%z")
        )
    }
}
