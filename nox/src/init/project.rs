use super::language::{Language, ProjectType};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub name: Option<String>,
    pub languages: BTreeSet<Language>,
    pub source_files: Vec<PathBuf>,
    pub header_files: Vec<PathBuf>,
    pub source_directories: BTreeSet<PathBuf>,
    pub test_directories: BTreeSet<PathBuf>,
    pub existing_files: BTreeSet<PathBuf>,
    pub package_manifests: Vec<PathBuf>,
    pub build_systems: Vec<String>,
    pub formatter_configs: Vec<PathBuf>,
    pub project_type: Option<ProjectType>,
    pub has_flake: bool,
    pub has_nix_formatter: bool,
    pub has_nox_build: bool,
    pub has_noxfile: bool,
    pub has_git: bool,
    pub has_readme: bool,
    pub has_tests: bool,
}
