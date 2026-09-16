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

use error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::rules::init::InitRules;
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
    let rules = InitRules::load().map_err(Error::Rules)?;
    let mut project = scanner::scan(root, &rules)?;
    detect::infer_type(&mut project, &rules);
    report(&project);
    let language = detect::language(&project, &rules, options.language.as_deref())?;
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
    new::create_files(
        root,
        &project,
        &rules,
        language,
        project_type,
        &options,
        &name,
    )?;
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

        let rules = InitRules::load().unwrap();
        let project = scanner::scan(&root, &rules).unwrap();
        assert_eq!(
            scanner::source_files(&project, Language::Cpp, &rules),
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

    #[test]
    fn rust_init_uses_the_project_name_in_generated_bin_manifest() {
        let root = temporary_root("rust-bin-name");
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

        let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("name = \"fixture\""));
        assert!(!manifest.contains("name = \"{name}\""));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn initialization_rules_are_loaded_and_drive_aliases_and_ignores() {
        let rules = InitRules::load().expect("embedded init rules should load");
        assert_eq!(
            Language::parse_with_rules("cpp", &rules),
            Some(Language::Cpp)
        );
        assert!(rules.ignores.iter().any(|rule| rule.path == ".cache"));
        assert_eq!(
            rules
                .target(Language::Cpp, ProjectType::Executable)
                .unwrap()
                .target,
            "cxx_executable"
        );
    }

    #[test]
    fn generated_cpp_properties_follow_rule_order() {
        let root = temporary_root("rules");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::create_dir_all(root.join("include")).unwrap();
        fs::write(root.join("src/main.cpp"), "").unwrap();
        fs::write(root.join("src/nested/util.cpp"), "").unwrap();
        fs::write(root.join("include/util.hpp"), "").unwrap();
        let rules = InitRules::load().unwrap();
        let project = scanner::scan(&root, &rules).unwrap();
        let generated = buildgen::render(
            &project,
            &rules,
            Language::Cpp,
            ProjectType::Executable,
            "fixture",
        );
        assert!(generated.contains("\"src/main.cpp\",\n        \"src/nested/util.cpp\""));
        assert!(generated.find("sources =").unwrap() < generated.find("include_dirs =").unwrap());
        assert!(generated.find("include_dirs =").unwrap() < generated.find("flags =").unwrap());
        assert!(generated.contains("\"-std=c++20\""));
        fs::remove_dir_all(root).unwrap();
    }
}
