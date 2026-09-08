mod build_system;
mod cli;
mod core;
mod project;
mod task;
mod toolchain;

use core::{error::Result, output};

fn main() {
    if let Err(error) = cli::run() {
        output::error(format!("nox: {error}"));
        std::process::exit(1);
    }
}

pub(crate) fn project_root() -> Result<std::path::PathBuf> {
    Ok(std::env::current_dir()?)
}
