use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(clap::Args, Debug)]
pub struct Args {
    #[clap(default_value = ".")]
    root: PathBuf,
}

pub fn execute(args: Args) -> Result<()> {
    let git = args.root.join(".git");
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
