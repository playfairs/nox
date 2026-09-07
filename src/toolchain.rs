use crate::error::Result;
use std::process::Command;

pub fn find(programs: &[&str]) -> String {
  programs
    .iter()
    .find(|program| Command::new(program).arg("--version").output().is_ok())
    .unwrap_or(&programs[0])
    .to_string()
}

pub fn detect_c() -> (String, String, String) {
  let compiler = find(&["cc", "clang", "gcc"]);
  let linker = compiler.clone();
  let archiver = find(&["ar", "llvm-ar"]);
  (compiler, linker, archiver)
}

pub fn detect_cpp() -> String {
  find(&["c++", "clang++", "g++"])
}

pub fn detect_rust() -> Result<String> {
  let rustc = find(&["rustc"]);
  let _ = Command::new(&rustc).arg("--version").output()?;
  Ok(rustc)
}
