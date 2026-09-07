use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Executable,
    StaticLibrary,
    SharedLibrary,
    RustExecutable,
    RustLibrary,
    DExecutable,
}

#[derive(Clone, Debug)]
pub struct Target {
    pub name: String,
    pub kind: TargetKind,
    pub sources: Vec<PathBuf>,
    pub dependencies: Vec<String>,
    pub include_dirs: Vec<PathBuf>,
    pub defines: Vec<String>,
    pub flags: Vec<String>,
    pub linker_flags: Vec<String>,
    pub install: bool,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub edition: String,
    pub dependencies: Vec<String>,
    pub targets: Vec<Target>,
}

impl Project {
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.iter().find(|target| target.name == name)
    }
}
