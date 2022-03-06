use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::database::{Blob, Object};
use crate::repository::Repository;

#[derive(clap::Args, Debug)]
pub struct Args {
    paths: Vec<PathBuf>,
}

pub fn execute(args: Args) -> Result<()> {
    let root_path = fs::canonicalize(".")?;
    let repo = Repository::new(root_path);

    let workspace = repo.workspace();
    let database = repo.database();
    let mut index = repo.index_for_update()?;

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
