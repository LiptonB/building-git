use std::collections::HashMap;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use super::object::Object;

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
            let first_parent = parents[0]
                .as_ref()
                .file_name()
                .ok_or(anyhow!("Missing filename in {:?}", parents[0].as_ref()))?
                .to_string_lossy()
                .into_owned();
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
