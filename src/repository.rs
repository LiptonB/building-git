use std::path::PathBuf;

use anyhow::Result;

use crate::database::Database;
use crate::index::Index;
use crate::refs::Refs;
use crate::workspace::Workspace;

pub struct Repository {
    git_path: PathBuf,
}

impl Repository {
    pub fn new(git_path: PathBuf) -> Self {
        Self { git_path }
    }

    pub fn database(&self) -> Database {
        let path = self.git_path.join("objects");
        Database::new(path)
    }

    pub fn index(&self) -> Result<Index> {
        Index::load(self.git_path.join("index"))
    }

    pub fn index_for_update(&self) -> Result<Index> {
        Index::load_for_update(self.git_path.join("index"))
    }

    pub fn refs(&self) -> Refs {
        Refs::new(self.git_path.clone())
    }

    pub fn workspace(&self) -> Workspace {
        Workspace::new(&self.git_path)
    }
}
