use std::env;
use std::fs;
use std::io::{self, Read};

use anyhow::{anyhow, Context, Result};
use rustc_serialize::hex::ToHex;
use time::OffsetDateTime;

use crate::database::{Author, Commit, Object, Tree, TreeFile};
use crate::repository::Repository;

#[derive(clap::Args, Debug)]
pub struct Args {}

pub fn execute(_args: Args) -> Result<()> {
    let root_path = fs::canonicalize(".")?;
    let repo = Repository::new(root_path);

    let index = repo.index()?;
    let refs = repo.refs();
    let database = repo.database();

    let entries = index
        .iter()
        .map(|entry| TreeFile::new(&entry.path, &entry.oid.to_hex(), entry.mode));

    let mut root = Tree::build(entries)?;
    root.traverse(&|tree| database.store(tree))?;

    let parent = refs.read_head()?;
    let name = env::var("GIT_AUTHOR_NAME").context("GIT_AUTHOR_NAME")?;
    let email = env::var("GIT_AUTHOR_EMAIL").context("GIT_AUTHOR_EMAIL")?;
    let timestamp = OffsetDateTime::try_now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
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
