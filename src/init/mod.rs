pub mod buildgen;
pub mod config;
pub mod detect;
pub mod error;
pub mod language;
pub mod layout;
pub mod new;
pub mod project;
pub mod scanner;
pub mod templates;

use config::ScanConfig;
use error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub use config::Options;
pub use language::{Language, ProjectType};
pub use project::ProjectInfo;

pub fn resolve_root(name: Option<&str>) -> Result<PathBuf> {
    let current = std::env::current_dir()?;
    let Some(name) = name else { return Ok(current) };
    let path = PathBuf::from(name);
    Ok(if path.is_absolute() {
        path
    } else {
        current.join(path)
    })
}

pub fn run(root: &Path, options: Options) -> Result<()> {
    if !root.exists() {
        fs::create_dir_all(root)?;
    }
    if !root.is_dir() {
        return Err(Error::InvalidDirectory(format!(
            "'{}' is not a directory",
            root.display()
        )));
    }
    crate::core::output::action("analyzing", root.display());
    let mut project = scanner::scan(root, &ScanConfig::default())?;
    detect::infer_type(&mut project);
    report(&project);
    let language = detect::language(&project, options.language.as_deref())?;
    let project_type = detect::project_type(&project, options.project_type.as_deref())?;
    let name = options
        .name
        .clone()
        .or(options.project_name.clone())
        .or(project.name.clone())
        .or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .ok_or_else(|| Error::Configuration("could not infer a project name; use --name".into()))?;
    new::create_files(root, &project, language, project_type, &options, &name)?;
    crate::core::output::success(format!("Nox project '{name}' initialized successfully"));
    Ok(())
}

fn report(project: &ProjectInfo) {
    for language in &project.languages {
        crate::core::output::action("detected", language.label());
    }
    if !project.package_manifests.is_empty() {
        crate::core::output::action(
            "detected",
            format!("{} package manifest(s)", project.package_manifests.len()),
        );
    }
    crate::core::output::action(
        "found",
        format!("{} source file(s)", project.source_files.len()),
    );
    if project.has_tests {
        crate::core::output::action("detected", "test suite");
    }
    if project.has_git {
        crate::core::output::action("detected", "Git repository");
    }
    if project.has_flake {
        crate::core::output::action("detected", "Nix flake");
    }
    if project.has_nox_build {
        crate::core::output::action("detected", "nox.build");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temporary_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nox-init-{label}-{}", std::process::id()))
    }

    #[test]
    fn scan_collects_sorted_nested_sources_and_skips_tests() {
        let root = temporary_root("recursive");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("src/nested/util.cpp"), "").unwrap();
        fs::write(root.join("src/main.cpp"), "").unwrap();
        fs::write(root.join("tests/test_main.cpp"), "").unwrap();

        let project = scanner::scan(&root, &ScanConfig::default()).unwrap();
        assert_eq!(
            scanner::source_files(&project, Language::Cpp),
            [
                std::path::PathBuf::from("src/main.cpp"),
                std::path::PathBuf::from("src/nested/util.cpp"),
            ]
        );
        assert!(
            project
                .test_directories
                .contains(std::path::Path::new("tests"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_init_generates_a_build_target_for_the_starter_source() {
        let root = temporary_root("empty");
        let _ = fs::remove_dir_all(&root);
        run(
            &root,
            Options {
                project_name: Some("fixture".into()),
                language: Some("rust".into()),
                nix: false,
                noxfile: false,
                ..Options::default()
            },
        )
        .unwrap();
        let build = fs::read_to_string(root.join("nox.build")).unwrap();
        assert!(build.contains("\"src/main.rs\""));
        fs::remove_dir_all(root).unwrap();
    }
}
