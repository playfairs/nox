use super::error::Result;
use super::language::{Language, is_test_path_with_rules};
use super::project::ProjectInfo;
use crate::rules::init::InitRules;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan(root: &Path, rules: &InitRules) -> Result<ProjectInfo> {
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
    visit(root, root, rules, &mut project)?;
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
    rules: &InitRules,
    project: &mut ProjectInfo,
) -> Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if path.is_dir() {
            if relative.components().any(|component| {
                rules.ignores.iter().any(|rule| {
                    rule.kind == "directory"
                        && component.as_os_str() == std::ffi::OsStr::new(&rule.path)
                })
            }) {
                continue;
            }
            if relative
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    rules.conventions.iter().any(|rule| {
                        rule.name == "test_directories"
                            && rule.values.iter().any(|value| value == name)
                    })
                })
            {
                project.test_directories.insert(relative.clone());
            }
            if rules.layouts.iter().any(|rule| {
                rule.role == "source"
                    && rule.directory
                        == relative
                            .file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or_default()
            }) {
                project.source_directories.insert(relative.clone());
            }
            visit(root, &path, rules, project)?;
            continue;
        }
        project.existing_files.insert(relative.clone());
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if let Some(rule) = rules.files.iter().find(|rule| rule.name == name) {
            for language in &rule.languages {
                project.languages.insert(*language);
            }
            if rule.build_system.as_deref() == Some("Nix") {
                project.has_flake = true;
            }
            if !rule.languages.is_empty() && rule.build_system.is_some() {
                project.package_manifests.push(relative.clone());
            }
            if let Some(build_system) = &rule.build_system {
                project.build_systems.push(build_system.clone());
            }
            if rule.project_name.as_deref() == Some("toml_name") {
                project.name = project.name.take().or_else(|| read_value(&path, "name"));
            }
            if rule.project_name.as_deref() == Some("json_name") {
                project.name = project.name.take().or_else(|| read_json_name(&path));
            }
            if rule.build_system.is_none() {
                project.formatter_configs.push(relative.clone());
            }
        }
        if let Some(language) = Language::from_path(&path, rules) {
            project.languages.insert(language);
            project.source_files.push(relative.clone());
            if let Some(parent) = relative.parent() {
                project.source_directories.insert(parent.to_path_buf());
            }
        }
        if Language::is_header(&path, rules) {
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
pub fn source_files(project: &ProjectInfo, language: Language, rules: &InitRules) -> Vec<PathBuf> {
    project
        .source_files
        .iter()
        .filter(|path| {
            Language::from_path(path, rules) == Some(language)
                && !is_test_path_with_rules(path, rules)
        })
        .cloned()
        .collect()
}
