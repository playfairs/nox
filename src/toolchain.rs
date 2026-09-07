use crate::error::{Error, Result};
use crate::rider::RiderKind;
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

pub fn detect_rider(kind: RiderKind) -> Result<String> {
    let candidates: &[&str] = match kind {
        RiderKind::Go => &["go"],
        RiderKind::Java => &["javac"],
        RiderKind::CSharp => &["csc", "mcs"],
        RiderKind::Swift => &["swiftc"],
        RiderKind::Zig => &["zig"],
        RiderKind::Python => &["python3", "python"],
        RiderKind::JavaScript => &["node"],
        RiderKind::TypeScript => &["tsc"],
        RiderKind::Kotlin => &["kotlinc"],
        RiderKind::C | RiderKind::Cpp | RiderKind::Rust => {
            return Err(Error::Config(
                "Rider uses the native toolchain detector".to_string(),
            ));
        }
    };
    candidates
        .iter()
        .find(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .status()
                .is_ok_and(|status| status.success())
        })
        .map(|candidate| (*candidate).to_string())
        .ok_or_else(|| Error::Config(format!("toolchain for {:?} Rider was not found", kind)))
}
