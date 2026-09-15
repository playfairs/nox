use super::project::ProjectInfo;
use std::path::{Path, PathBuf};
pub fn include_directories(root: &Path, project: &ProjectInfo) -> Vec<PathBuf> {
    let mut directories = project
        .header_files
        .iter()
        .map(|path| {
            let components = path.components().collect::<Vec<_>>();
            if let Some(index) = components.iter().position(|component| {
                matches!(
                    component.as_os_str().to_str(),
                    Some("include" | "includes" | "inc" | "headers" | "public")
                )
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
