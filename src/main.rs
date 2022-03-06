mod cmd;
mod database;
mod index;
mod lockfile;
mod refs;
mod repository;
mod telemetry;
mod workspace;

use anyhow::Result;

fn main() -> Result<()> {
    telemetry::init();

    let args = std::env::args_os();
    cmd::execute(args)
}
