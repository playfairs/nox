use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Setting {
    String(String),
    List(Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetLanguage {
    Rust,
    Haskell,
    C,
    Cpp,
    D,
    Swift,
    FSharp,
    JavaScript,
    TypeScript,
    Python,
    Kotlin,
    Gradle,
}

impl TargetLanguage {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "rust" => Some(Self::Rust),
            "haskell" => Some(Self::Haskell),
            "c" => Some(Self::C),
            "cpp" | "c++" | "cxx" => Some(Self::Cpp),
            "d" => Some(Self::D),
            "swift" => Some(Self::Swift),
            "fsharp" | "f#" => Some(Self::FSharp),
            "javascript" | "js" => Some(Self::JavaScript),
            "typescript" | "ts" => Some(Self::TypeScript),
            "python" | "py" => Some(Self::Python),
            "kotlin" | "kt" => Some(Self::Kotlin),
            "gradle" => Some(Self::Gradle),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Executable { language: Option<TargetLanguage> },
    StaticLibrary,
    SharedLibrary,
    RustLibrary,
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
    pub gradle_tasks: Vec<String>,
    pub gradle_options: Vec<String>,
    pub gradle_run_tasks: Vec<String>,
    pub gradle_run_options: Vec<String>,
    pub install: bool,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub version: Option<String>,
    pub version_files: Option<Vec<PathBuf>>,
    pub description: String,
    pub license: String,
    pub edition: String,
    pub dependencies: Vec<String>,
    pub repository: Option<String>,
    pub website: Option<String>,
    pub authors: Vec<String>,
    pub maintainers: Vec<String>,
    pub targets: Vec<Target>,
    pub settings: HashMap<String, Setting>,
    pub extra_env: HashMap<String, String>,
}

impl Project {
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.iter().find(|target| target.name == name)
    }
}
