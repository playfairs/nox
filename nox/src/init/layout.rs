use super::project::ProjectInfo;
use crate::rules::init::InitRules;
use std::path::{Path, PathBuf};
pub fn include_directories(root: &Path, project: &ProjectInfo, rules: &InitRules) -> Vec<PathBuf> {
    let include_names = rules
        .layouts
        .iter()
        .filter(|rule| rule.role == "include")
        .map(|rule| rule.directory.as_str())
        .collect::<Vec<_>>();
    let mut directories = project
        .header_files
        .iter()
        .map(|path| {
            let components = path.components().collect::<Vec<_>>();
            if let Some(index) = components.iter().position(|component| {
                component
                    .as_os_str()
                    .to_str()
                    .is_some_and(|value| include_names.contains(&value))
            }) {
                components[..=index]
                    .iter()
                    .fold(PathBuf::new(), |directory, component| {
                        directory.join(component.as_os_str())
                    })
            } else {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            }
        })
        .map(|path| root.join(path))
        .collect::<Vec<_>>();
    directories.sort();
    directories.dedup();
    directories
}
