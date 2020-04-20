mod database;
mod workspace;

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

use database::{Blob, Database};
use workspace::Workspace;

#[derive(StructOpt, Debug)]
enum Opt {
    Init {
        #[structopt(default_value = ".")]
        root: PathBuf,
    },
    Commit,
}

fn init<P: AsRef<Path>>(root: P) -> Result<()> {
    let root = fs::canonicalize(root)?;
    let git = root.join(".git");

    let create = |dirname| {
        let path = git.join(dirname);
        fs::create_dir_all(&path).with_context(|| format!("Failed to create {}", path.display()))
    };
    create("objects")?;
    create("refs")?;
    println!("Initialized empty Jit repository in {}", git.display());
    Ok(())
}

fn commit() -> Result<()> {
    let root_path = fs::canonicalize(".")?;
    let db_path = root_path.join(".git").join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(&db_path);

    for file in workspace.list_files()? {
        let data = file.read()?;
        let blob = Box::new(Blob::new(data));
        database.store(blob);
    }

    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    match opt {
        Opt::Init { root } => init(&root)?,
        Opt::Commit => commit()?,
    }

    Ok(())
}
