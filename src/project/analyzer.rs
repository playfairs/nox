use crate::core::error::Result;
use crate::init::detect;
use crate::init::language::is_test_path;
use crate::init::scanner;
use crate::rules::init::InitRules;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub use crate::init::language::{Language, ProjectType};

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
    let rules = InitRules::load().map_err(crate::core::error::Error::Config)?;
    let mut project = scanner::scan(root, &rules)
        .map_err(|error| crate::core::error::Error::Config(error.to_string()))?;
    detect::infer_type(&mut project, &rules);
    Ok(Analysis {
        languages: project.languages,
        source_files: project.source_files,
        header_files: project.header_files,
        source_directories: project.source_directories,
        test_directories: project.test_directories,
        project_name: project.name,
        project_type: project.project_type,
        package_manifests: project.package_manifests,
        build_systems: project.build_systems,
        formatter_configs: project.formatter_configs,
        has_flake: project.has_flake,
        has_nix_directory: root.join("nix").is_dir(),
        has_nix_formatter: project.has_nix_formatter,
        has_nox_build: project.has_nox_build,
        has_noxfile: project.has_noxfile,
        has_git: project.has_git,
        has_readme: project.has_readme,
        has_license: root.join("LICENSE").exists() || root.join("LICENCE").exists(),
        has_tests: project.has_tests,
    })
}

pub fn is_test_path_legacy(path: &Path) -> bool {
    is_test_path(path)
}
