mod add;
mod commit;
mod init;

use std::ffi::OsString;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
enum Cli {
    Init(init::Args),
    Commit(commit::Args),
    Add(add::Args),
}

pub fn execute<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let opt = Cli::parse_from(args);
    match opt {
        Cli::Init(args) => init::execute(args),
        Cli::Commit(args) => commit::execute(args),
        Cli::Add(args) => add::execute(args),
    }
}
