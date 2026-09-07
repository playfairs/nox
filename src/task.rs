use crate::error::{Error, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_task(path: &Path, name: &str) -> Result<()> {
  let text = fs::read_to_string(path)?;
  let marker = format!("task \"{name}\"");
  let start = text
    .find(&marker)
    .ok_or_else(|| Error::Config(format!("task '{name}' was not found")))?;
  let block = &text[start..];
  let command = block
    .split("run = \"")
    .nth(1)
    .and_then(|value| value.split('"').next())
    .ok_or_else(|| Error::Config(format!("task '{name}' has no run command")))?;
  let status = if cfg!(windows) {
    Command::new("cmd").args(["/C", command]).status()?
  } else {
    Command::new("sh").args(["-c", command]).status()?
  };
  if status.success() {
    Ok(())
  } else {
    Err(Error::Process(format!(
      "task '{name}' exited with {status}"
    )))
  }
}
