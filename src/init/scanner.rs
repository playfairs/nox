use super::config::ScanConfig;
use super::error::Result;
use super::language::{Language, is_test_path};
use super::project::ProjectInfo;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan(root: &Path, config: &ScanConfig) -> Result<ProjectInfo> {
    let mut project = ProjectInfo {
        root: root.to_path_buf(),
        has_flake: root.join("flake.nix").is_file(),
        has_nix_formatter: root.join("nix/formatter.nix").is_file(),
        has_nox_build: root.join("nox.build").is_file(),
        has_noxfile: root.join("noxfile").is_file(),
        has_git: root.join(".git").exists(),
        has_readme: root.join("README").exists() || root.join("README.md").exists(),
        ..ProjectInfo::default()
    };
    visit(root, root, config, &mut project)?;
    if project.languages.contains(&Language::JavaScript)
        && project.existing_files.contains(Path::new("tsconfig.json"))
    {
        project.languages.remove(&Language::JavaScript);
        project.languages.insert(Language::TypeScript);
    }
    project.source_files.sort();
    project.header_files.sort();
    project.package_manifests.sort();
    project.formatter_configs.sort();
    project.has_tests = !project.test_directories.is_empty();
    Ok(project)
}
fn visit(
    root: &Path,
    directory: &Path,
    config: &ScanConfig,
    project: &mut ProjectInfo,
) -> Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if path.is_dir() {
            if relative.components().any(|component| {
                config
                    .ignored_directories
                    .contains(component.as_os_str().to_str().unwrap_or_default())
            }) {
                continue;
            }
            if relative
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| matches!(name, "test" | "tests" | "spec" | "__tests__"))
            {
                project.test_directories.insert(relative.clone());
            }
            visit(root, &path, config, project)?;
            continue;
        }
        project.existing_files.insert(relative.clone());
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        match name {
            "Cargo.toml" => {
                project.languages.insert(Language::Rust);
                project.package_manifests.push(relative.clone());
                project.build_systems.push("Cargo".into());
                project.name = project.name.take().or_else(|| read_value(&path, "name"));
            }
            "Package.swift" => {
                project.languages.insert(Language::Swift);
                project.package_manifests.push(relative.clone());
                project.build_systems.push("SwiftPM".into());
            }
            "package.json" => {
                project.languages.insert(Language::JavaScript);
                project.package_manifests.push(relative.clone());
                project.build_systems.push("npm".into());
                project.name = project.name.take().or_else(|| read_json_name(&path));
            }
            "pyproject.toml" => {
                project.languages.insert(Language::Python);
                project.package_manifests.push(relative.clone());
                project.build_systems.push("pyproject".into());
            }
            "tsconfig.json" => {
                project.languages.insert(Language::TypeScript);
                project.package_manifests.push(relative.clone());
            }
            "CMakeLists.txt" => project.build_systems.push("CMake".into()),
            "meson.build" => project.build_systems.push("Meson".into()),
            "Makefile" => project.build_systems.push("Make".into()),
            "flake.nix" => project.has_flake = true,
            "rustfmt.toml" | ".clang-format" | ".prettierrc" | ".prettierrc.json"
            | ".swift-format" => project.formatter_configs.push(relative.clone()),
            _ => {}
        }
        if let Some(language) = Language::from_path(&path) {
            project.languages.insert(language);
            project.source_files.push(relative.clone());
            if let Some(parent) = relative.parent() {
                project.source_directories.insert(parent.to_path_buf());
            }
        }
        if Language::is_header(&path) {
            project.header_files.push(relative);
        }
    }
    Ok(())
}
fn read_value(path: &Path, key: &str) -> Option<String> {
    fs::read_to_string(path).ok()?.lines().find_map(|line| {
        line.strip_prefix(&format!("{key} = \""))
            .and_then(|value| value.strip_suffix('"'))
            .map(str::to_string)
    })
}
fn read_json_name(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()?.lines().find_map(|line| {
        line.trim()
            .strip_prefix("\"name\"")?
            .split_once(':')
            .map(|(_, value)| value.trim().trim_matches(',').trim_matches('"').to_string())
    })
}
pub fn source_files(project: &ProjectInfo, language: Language) -> Vec<PathBuf> {
    project
        .source_files
        .iter()
        .filter(|path| Language::from_path(path) == Some(language) && !is_test_path(path))
        .cloned()
        .collect()
}
