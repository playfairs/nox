use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct Options {
    pub project_name: Option<String>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub project_type: Option<String>,
    pub template: Option<String>,
    pub nix: bool,
    pub formatter: bool,
    pub noxfile: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            project_name: None,
            name: None,
            language: None,
            project_type: None,
            template: None,
            nix: true,
            formatter: false,
            noxfile: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScanConfig {
    pub ignored_directories: BTreeSet<&'static str>,
}
impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            ignored_directories: [".git", "target", "build", "node_modules", "dist", ".nox"]
                .into_iter()
                .collect(),
        }
    }
}
