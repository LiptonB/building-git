mod database;
mod index;
mod lockfile;
mod refs;
mod workspace;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use rustc_serialize::hex::ToHex;
use structopt::StructOpt;
use time::OffsetDateTime;

use crate::database::*;
use crate::index::*;
use crate::refs::*;
use crate::workspace::*;

#[derive(StructOpt, Debug)]
enum Opt {
    Init {
        #[structopt(default_value = ".")]
        root: PathBuf,
    },
    Commit,
    Add {
        paths: Vec<PathBuf>,
    },
}

fn init<P: AsRef<Path>>(root: P) -> Result<()> {
    let git = root.as_ref().join(".git");
    fs::create_dir_all(&git).with_context(|| format!("Failed to create {}", git.display()))?;
    let git = fs::canonicalize(git)?;

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
    let git_path = root_path.join(".git");
    let db_path = git_path.join("objects");

    let index = Index::load(git_path.join("index"))?;
    let refs = Refs::new(git_path);
    let database = Database::new(&db_path);

    let entries = index
        .iter()
        .map(|entry| TreeFile::new(&entry.path, &entry.oid.to_hex(), entry.mode));

    let mut root = Tree::build(entries)?;
    root.traverse(&|tree| database.store(tree))?;

    let parent = refs.read_head()?;
    let name = env::var("GIT_AUTHOR_NAME").context("GIT_AUTHOR_NAME")?;
    let email = env::var("GIT_AUTHOR_EMAIL").context("GIT_AUTHOR_EMAIL")?;
    let timestamp = OffsetDateTime::try_now_local().unwrap_or_else(|_| { OffsetDateTime::now_utc() });
    let author = Author::new(&name, &email, timestamp);

    let mut message = String::new();
    io::stdin().read_to_string(&mut message)?;

    let mut commit = Commit::new(
        parent.to_owned(),
        root.oid().to_owned(),
        author,
        message.clone(),
    );
    database.store(&mut commit)?;

    let first_line = message.lines().next().ok_or(anyhow!("Empty message"))?;
    let commit_oid = commit.oid();

    refs.update_head(commit_oid)?;

    let is_root = if parent.is_none() {
        "(root-commit) "
    } else {
        ""
    };
    println!("[{}{}] {}", is_root, commit_oid, first_line);

    Ok(())
}

fn add(paths: Vec<PathBuf>) -> Result<()> {
    let root_path = fs::canonicalize(".")?;
    let git_path = root_path.join(".git");

    let workspace = Workspace::new(root_path);
    let database = Database::new(git_path.join("objects"));
    let mut index = Index::load_for_update(git_path.join("index"))?;

    //index.load_for_update()?;

    for path in paths {
        for file in workspace.list_files(path)? {
            let data = file.read()?;
            let metadata = file.stat()?;

            let mut blob = Blob::new(data);
            database.store(&mut blob)?;
            index.add(&file, blob.oid(), &metadata);
        }
    }

    index.write_updates()?;

    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    match opt {
        Opt::Init { root } => init(&root)?,
        Opt::Commit => commit()?,
        Opt::Add { paths } => add(paths)?,
    }

    Ok(())
}

/*
fn main() {
    let result = do_main();

    if let Err(err) = result {

    }
}
*/
