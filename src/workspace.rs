use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

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

    fn path<P: AsRef<Path>>(&self, path: P) -> Result<WorkspacePath> {
        let full_path = path.as_ref().canonicalize()?;
        let rel_path = full_path
            .strip_prefix(&self.root)
            .with_context(|| format!("Path {} is not inside workspace", full_path.display()))?;
        Ok(WorkspacePath {
            workspace: self,
            rel_path: rel_path.to_owned(),
        })
    }

    pub fn list_files(&self) -> Result<Vec<WorkspacePath>> {
        let mut results = Vec::new();
        self.list_files_in(self.path(".")?, &mut results)?;
        Ok(results)
    }

    fn list_files_in<'a>(
        &'a self,
        dir: WorkspacePath,
        results: &mut Vec<WorkspacePath<'a>>,
    ) -> Result<()> {
        const IGNORE: &[&str] = &[".git", ".swp", ".un~", "target"];
        for entry in dir.path().read_dir()? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or(anyhow!("Invalid filename found"))?;
            if IGNORE.iter().any(|ig| name.contains(ig)) {
                continue;
            }
            if entry.file_type()?.is_dir() {
                self.list_files_in(self.path(entry.path())?, results)?;
            } else {
                results.push(self.path(entry.path())?);
            }
        }
        Ok(())
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

    pub fn stat(&self) -> Result<fs::Metadata> {
        let metadata = fs::metadata(self.path())?;
        Ok(metadata)
    }
}
