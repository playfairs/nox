use crate::core::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub fn run(root: &Path, arguments: &[String]) -> Result<()> {
    #[cfg(windows)]
    let wrapper = root.join("gradlew.bat");
    #[cfg(not(windows))]
    let wrapper = root.join("gradlew");

    let mut command = if wrapper.is_file() {
        #[cfg(windows)]
        {
            let mut command = Command::new("cmd");
            command.args(["/C"]).arg(&wrapper);
            command
        }
        #[cfg(not(windows))]
        {
            Command::new(&wrapper)
        }
    } else {
        Command::new("gradle")
    };
    let status = command
        .current_dir(root)
        .args(arguments)
        .status()
        .map_err(|error| {
            Error::Process(format!(
                "could not start Gradle in '{}': {error}",
                root.display()
            ))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Exit(status.code().unwrap_or(1)))
    }
}