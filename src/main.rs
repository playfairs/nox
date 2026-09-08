mod build_system;
mod cli;
mod core;
mod project;
mod run;
mod task;
mod toolchain;

use core::{error::Error, output};

fn main() {
    match cli::run() {
        Ok(()) => {}
        Err(Error::Exit(code)) => std::process::exit(code),
        Err(error) => {
            output::error(format!("nox: {error}"));
            std::process::exit(1);
        }
    }
}

pub(crate) fn project_root() -> core::error::Result<std::path::PathBuf> {
    Ok(std::env::current_dir()?)
}
