mod cmd;
mod database;
mod index;
mod lockfile;
mod refs;
mod repository;
mod telemetry;
mod workspace;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
enum Cli {
    Init(cmd::init::Args),
    Commit(cmd::commit::Args),
    Add(cmd::add::Args),
}

fn main() -> Result<()> {
    telemetry::init();

    let opt = Cli::parse();
    match opt {
        Cli::Init(args) => cmd::init::execute(args)?,
        Cli::Commit(args) => cmd::commit::execute(args)?,
        Cli::Add(args) => cmd::add::execute(args)?,
    }

    Ok(())
}
