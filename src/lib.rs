pub mod build_system;
pub mod cli;
pub mod core;
pub mod init;
pub mod project;
pub mod run;
pub mod task;
pub mod toolchain;

pub fn project_root() -> core::error::Result<std::path::PathBuf> {
    let path = std::env::current_dir()?;
    if path.join("nox.build").is_file() {
        return Ok(path);
    }
    let Some(parent) = path.parent() else {
        return Err(core::error::Error::Config(
            "could not find a 'nox.build' file".to_string(),
        ));
    };
    core::output::warning(format!(
        "path '{}' does not contain a 'nox.build', searching up",
        path.display()
    ));
    if parent.join("nox.build").is_file() {
        return Ok(parent.to_path_buf());
    }
    Err(core::error::Error::Config(
        "could not find a 'nox.build' file".to_string(),
    ))
}
