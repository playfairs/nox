use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Language {
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
    Unknown,
}
impl Language {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "rust" => Some(Self::Rust),
            "haskell" | "hs" => Some(Self::Haskell),
            "c" => Some(Self::C),
            "cpp" | "c++" | "cxx" => Some(Self::Cpp),
            "d" => Some(Self::D),
            "swift" => Some(Self::Swift),
            "fsharp" | "f#" => Some(Self::FSharp),
            "javascript" | "js" => Some(Self::JavaScript),
            "typescript" | "ts" => Some(Self::TypeScript),
            "python" | "py" => Some(Self::Python),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Haskell => "Haskell",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::D => "D",
            Self::Swift => "Swift",
            Self::FSharp => "F#",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
            Self::Unknown => "unknown language",
        }
    }
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "hs" | "lhs" => Some(Self::Haskell),
            "c" => Some(Self::C),
            "cc" | "cpp" | "cxx" => Some(Self::Cpp),
            "d" => Some(Self::D),
            "swift" => Some(Self::Swift),
            "fsx" | "fs" => Some(Self::FSharp),
            "js" | "jsx" | "mjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "py" => Some(Self::Python),
            _ => None,
        }
    }
    pub fn is_header(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("h" | "hh" | "hpp" | "hxx")
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectType {
    Executable,
    Library,
}
impl ProjectType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "executable" | "exe" | "binary" | "bin" => Some(Self::Executable),
            "library" | "lib" => Some(Self::Library),
            _ => None,
        }
    }
}
pub fn is_test_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some("test" | "tests" | "spec" | "__tests__")
        )
    })
}
