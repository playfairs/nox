use crate::core::error::Result;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Language {
    Rust,
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

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub languages: BTreeSet<Language>,
    pub source_files: Vec<PathBuf>,
    pub header_files: Vec<PathBuf>,
    pub source_directories: BTreeSet<PathBuf>,
    pub test_directories: BTreeSet<PathBuf>,
    pub project_name: Option<String>,
    pub project_type: Option<ProjectType>,
    pub package_manifests: Vec<PathBuf>,
    pub build_systems: Vec<String>,
    pub formatter_configs: Vec<PathBuf>,
    pub has_flake: bool,
    pub has_nix_directory: bool,
    pub has_nix_formatter: bool,
    pub has_nox_build: bool,
    pub has_noxfile: bool,
    pub has_git: bool,
    pub has_readme: bool,
    pub has_license: bool,
    pub has_tests: bool,
}

pub fn analyze(root: &Path) -> Result<Analysis> {
    let mut analysis = Analysis {
        has_flake: root.join("flake.nix").is_file(),
        has_nix_directory: root.join("nix").is_dir(),
        has_nix_formatter: root.join("nix/formatter.nix").is_file(),
        has_nox_build: root.join("nox.build").is_file(),
        has_noxfile: root.join("noxfile").is_file(),
        has_git: root.join(".git").exists(),
        has_readme: root.join("README").exists() || root.join("README.md").exists(),
        has_license: root.join("LICENSE").exists() || root.join("LICENCE").exists(),
        ..Analysis::default()
    };
    scan(root, root, &mut analysis)?;
    if root.join("tsconfig.json").is_file() {
        analysis.languages.remove(&Language::JavaScript);
        analysis.languages.insert(Language::TypeScript);
    }
    analysis.source_files.sort();
    analysis.header_files.sort();
    analysis.package_manifests.sort();
    analysis.formatter_configs.sort();
    analysis.has_tests = !analysis.test_directories.is_empty();
    if analysis.project_type.is_none() && !analysis.source_files.is_empty() {
        analysis.project_type = Some(
            if analysis.source_files.iter().any(|path| {
                path.file_name().and_then(|name| name.to_str()) == Some("main.rs")
                    || path.file_name().and_then(|name| name.to_str()) == Some("main.c")
                    || path.file_name().and_then(|name| name.to_str()) == Some("main.cpp")
                    || path.file_name().and_then(|name| name.to_str()) == Some("main.swift")
            }) {
                ProjectType::Executable
            } else {
                ProjectType::Library
            },
        );
    }
    Ok(analysis)
}

fn scan(root: &Path, directory: &Path, analysis: &mut Analysis) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if path.is_dir() {
            if relative.components().any(|component| {
                matches!(
                    component.as_os_str().to_str(),
                    Some(".git" | "target" | "build" | "node_modules")
                )
            }) {
                continue;
            }
            if relative
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name == "test" || name == "tests" || name == "spec" || name == "__tests__"
                })
            {
                analysis.test_directories.insert(relative.to_path_buf());
            }
            scan(root, &path, analysis)?;
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        match name {
            "Cargo.toml" => {
                analysis.languages.insert(Language::Rust);
                analysis.package_manifests.push(relative.to_path_buf());
                analysis.build_systems.push("Cargo".to_string());
                if let Some(value) = read_value(&path, "name") {
                    analysis.project_name.get_or_insert(value);
                }
                if fs::read_to_string(&path).is_ok_and(|text| text.contains("[[bin]]")) {
                    analysis.project_type = Some(ProjectType::Executable);
                }
            }
            "package.json" => {
                analysis.languages.insert(Language::JavaScript);
                analysis.package_manifests.push(relative.to_path_buf());
                analysis.build_systems.push("npm".to_string());
                if let Some(value) = read_json_name(&path) {
                    analysis.project_name.get_or_insert(value);
                }
            }
            "tsconfig.json" => {
                analysis.languages.insert(Language::TypeScript);
                analysis.package_manifests.push(relative.to_path_buf());
            }
            "CMakeLists.txt" => analysis.build_systems.push("CMake".to_string()),
            "meson.build" => analysis.build_systems.push("Meson".to_string()),
            "Makefile" => analysis.build_systems.push("Make".to_string()),
            "Justfile" => analysis.build_systems.push("Just".to_string()),
            "Package.swift" => {
                analysis.languages.insert(Language::Swift);
                analysis.package_manifests.push(relative.to_path_buf());
                analysis.build_systems.push("SwiftPM".to_string());
            }
            "build.zig" => analysis.build_systems.push("Zig".to_string()),
            "go.mod" => analysis.build_systems.push("Go".to_string()),
            "pyproject.toml" => {
                analysis.languages.insert(Language::Python);
                analysis.package_manifests.push(relative.to_path_buf());
                analysis.build_systems.push("pyproject".to_string());
            }
            "flake.nix" => analysis.has_flake = true,
            "rustfmt.toml" | ".clang-format" | ".prettierrc" | ".prettierrc.json"
            | ".swift-format" => {
                analysis.formatter_configs.push(relative.to_path_buf());
            }
            _ => {}
        }
        let language = match extension {
            "rs" => Some(Language::Rust),
            "c" => Some(Language::C),
            "cc" | "cpp" | "cxx" | "hpp" => Some(Language::Cpp),
            "d" => Some(Language::D),
            "swift" => Some(Language::Swift),
            "fsx" | "fs" => Some(Language::FSharp),
            "js" | "jsx" | "mjs" => Some(Language::JavaScript),
            "ts" | "tsx" => Some(Language::TypeScript),
            "py" => Some(Language::Python),
            _ => None,
        };
        if let Some(language) = language {
            analysis.languages.insert(language);
            if matches!(
                extension,
                "c" | "cc"
                    | "cpp"
                    | "cxx"
                    | "rs"
                    | "d"
                    | "swift"
                    | "fsx"
                    | "fs"
                    | "js"
                    | "jsx"
                    | "mjs"
                    | "ts"
                    | "tsx"
                    | "py"
            ) {
                analysis.source_files.push(relative.to_path_buf());
                if let Some(parent) = relative.parent() {
                    analysis.source_directories.insert(parent.to_path_buf());
                }
            }
        }
        if matches!(extension, "h" | "hh" | "hpp" | "hxx") {
            analysis.header_files.push(relative.to_path_buf());
        }
    }
    analysis
        .languages
        .retain(|language| *language != Language::Unknown);
    Ok(())
}

fn read_value(path: &Path, key: &str) -> Option<String> {
    fs::read_to_string(path).ok()?.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().trim_matches('"').to_string())
    })
}

fn read_json_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim().trim_matches('"') == "name")
            .then(|| value.trim().trim_matches(',').trim_matches('"').to_string())
    })
}
