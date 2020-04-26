pub mod database;
pub mod workspace;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use structopt::StructOpt;
use time::OffsetDateTime;

use database::*;
use workspace::*;

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
    let git_path = root_path.join(".git");
    let db_path = git_path.join("objects");

    let workspace = Workspace::new(&root_path);
    let database = Database::new(&db_path);

    let mut entries = Vec::new();
    for file in workspace.list_files()? {
        let data = file.read()?;
        let blob = Blob::new(data);
        database.store(&blob)?;

        let metadata = file.stat()?;
        entries.push(TreeFile::new(file.rel_path(), &blob.oid(), &metadata));
    }

    let root = Tree::build(entries)?;
    root.traverse(&|tree| database.store(tree))?;

    let name = env::var("GIT_AUTHOR_NAME").context("GIT_AUTHOR_NAME")?;
    let email = env::var("GIT_AUTHOR_EMAIL").context("GIT_AUTHOR_EMAIL")?;
    let author = Author::new(&name, &email, OffsetDateTime::now_local());

    let mut message = String::new();
    io::stdin().read_to_string(&mut message)?;

    let commit = Commit::new(&root.oid(), author, &message);
    database.store(&commit)?;

    let first_line = message.lines().next().ok_or(anyhow!("Empty message"))?;
    let commit_oid = commit.oid();

    fs::write(git_path.join("HEAD"), &commit_oid)?;

    println!("[(root-commit) {}] {}", commit_oid, first_line);

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

/*
fn main() {
    let result = do_main();

    if let Err(err) = result {

    }
}
*/
