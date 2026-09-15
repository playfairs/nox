use super::language::{Language, ProjectType};
use super::layout::include_directories;
use super::project::ProjectInfo;

pub fn render(
    project: &ProjectInfo,
    language: Language,
    project_type: ProjectType,
    name: &str,
) -> String {
    let mut source_paths = super::scanner::source_files(project, language);
    if source_paths.is_empty() {
        source_paths.push(std::path::PathBuf::from(
            super::templates::starter(language, project_type).0,
        ));
    }
    let sources = source_paths
        .into_iter()
        .map(|path| format!("        \"{}\"", path.to_string_lossy().replace('\\', "/")))
        .collect::<Vec<_>>()
        .join(",\n");
    let includes = include_directories(&project.root, project)
        .into_iter()
        .map(|path| {
            path.strip_prefix(&project.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .map(|path| format!("        \"{path}\""))
        .collect::<Vec<_>>();
    let include_block = if includes.is_empty() {
        String::new()
    } else {
        format!(
            "\n        include_dirs = [\n{}\n        ]",
            includes.join(",\n")
        )
    };
    let target = match (language, project_type) {
        (Language::Rust, ProjectType::Library) => "rust_library",
        (Language::Rust, ProjectType::Executable) => "rust_executable",
        (Language::Cpp, ProjectType::Executable) => "cxx_executable",
        (Language::C, ProjectType::Library) => "static_library",
        _ => "executable",
    };
    let flags = if language == Language::Cpp {
        "\n        flags = [\n        \"-std=c++20\"\n        ]"
    } else {
        ""
    };
    format!(
        "project \"{name}\" {{\n    description = \"A project built with Nox.\"\n\n    {target} \"{name}\" {{\n        sources = [\n{sources}\n        ]{include_block}{flags}\n        install = true\n    }}\n}}\n"
    )
}
