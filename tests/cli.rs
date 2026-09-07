use std::process::Command;

#[test]
fn binary_reports_help() {
  let binary = env!("CARGO_BIN_EXE_nox");
  let output = Command::new(binary).arg("help").output().unwrap();
  assert!(output.status.success());
  assert!(String::from_utf8_lossy(&output.stdout).contains("The Nox Build System"));
}
