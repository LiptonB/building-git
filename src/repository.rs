use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::database::Database;
use crate::index::Index;
use crate::refs::Refs;
use crate::workspace::Workspace;

struct Repository {
    git_path: PathBuf,
    database: Option<Database>,
}

impl Repository {
    pub fn database(&self) -> Database {
        let path = self.git_path.join("objects");
        Database::new(path)
    }

    pub fn index(&self) -> Result<Index> {
        Index::load(self.git_path.join("index"))
    }

    pub fn refs(&self) -> Refs {
        Refs::new(self.git_path.clone())
    }

    pub fn workspace(&self) -> Workspace {
        Workspace::new(&self.git_path)
    }
}
