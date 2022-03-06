use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::database::{Blob, Database, Object};
use crate::index::Index;
use crate::workspace::Workspace;

#[derive(clap::Args, Debug)]
pub struct Args {
    paths: Vec<PathBuf>,
}

pub fn execute(args: Args) -> Result<()> {
    let root_path = fs::canonicalize(".")?;
    let git_path = root_path.join(".git");

    let workspace = Workspace::new(root_path);
    let database = Database::new(git_path.join("objects"));
    let mut index = Index::load_for_update(git_path.join("index"))?;

    let files = args
        .paths
        .iter()
        .map(|path| workspace.list_files(path))
        .collect::<Result<Vec<_>>>()?;
    let files = files.iter().flatten();

    for file in files {
        let data = file.read()?;

        let mut blob = Blob::new(data);
        database.store(&mut blob)?;
        index.add(&file, blob.oid())?;
    }

    index.write_updates()?;

    Ok(())
}
