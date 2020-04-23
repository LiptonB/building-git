use anyhow::{anyhow, Result};
use maplit::hashset;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
}

#[derive(Debug)]
pub struct WorkspacePath<'a> {
    workspace: &'a Workspace,
    rel_path: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root: path.as_ref().to_owned(),
        }
    }

    fn path<P: AsRef<Path>>(&self, path: P) -> WorkspacePath {
        let full_path = path.as_ref();
        let rel_path = full_path
            .strip_prefix(&self.root)
            .expect("Path is not inside workspace");
        WorkspacePath {
            workspace: self,
            rel_path: rel_path.to_owned(),
        }
    }

    pub fn list_files(&self) -> Result<Vec<WorkspacePath>> {
        let ignore = hashset! {".git", ".swp", ".un~"};
        let mut results = Vec::new();
        for entry in self.root.read_dir()? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or(anyhow!("Invalid filename found"))?;
            if ignore.iter().any(|ig| name.contains(ig)) {
                continue;
            }
            if !entry.file_type()?.is_file() {
                continue;
            }
            results.push(self.path(entry.path()));
        }
        Ok(results)
    }
}

impl WorkspacePath<'_> {
    fn path(&self) -> PathBuf {
        self.workspace.root.join(&self.rel_path)
    }

    pub fn rel_path(&self) -> &PathBuf {
        &self.rel_path
    }

    pub fn read(&self) -> Result<Vec<u8>> {
        let data = fs::read(self.path())?;
        Ok(data)
    }
}
