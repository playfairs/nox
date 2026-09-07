mod cli;
mod error;
mod executor;
mod graph;
mod model;
mod parser;
mod rider;
mod state;
mod task;
mod toolchain;

use error::Result;

fn main() {
    if let Err(error) = cli::run() {
        eprintln!("nox: {error}");
        std::process::exit(1);
    }
}

pub(crate) fn project_root() -> Result<std::path::PathBuf> {
    Ok(std::env::current_dir()?)
}
