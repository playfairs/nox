pub mod build_system;
pub mod cli;
pub mod core;
pub mod project;
pub mod run;
pub mod task;
pub mod toolchain;

pub fn project_root() -> core::error::Result<std::path::PathBuf> {
    Ok(std::env::current_dir()?)
}
