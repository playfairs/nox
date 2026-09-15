use super::language::{Language, ProjectType};
use super::layout::include_directories;
use super::project::ProjectInfo;
use crate::rules::init::InitRules;

pub fn render(
    project: &ProjectInfo,
    rules: &InitRules,
    language: Language,
    project_type: ProjectType,
    name: &str,
) -> String {
    let mut source_paths = super::scanner::source_files(project, language, rules);
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
    let sources_block = format!("        sources = [\n{sources}\n        ]");
    let includes = include_directories(&project.root, project, rules)
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
    let target = rules
        .target(language, project_type)
        .map(|rule| rule.target.as_str())
        .unwrap_or("executable");
    let default_flags = rules
        .flags
        .iter()
        .filter(|rule| rule.languages.contains(&language))
        .flat_map(|rule| rule.values.iter())
        .cloned()
        .collect::<Vec<_>>();
    let flags = if default_flags.is_empty() {
        String::new()
    } else {
        format!(
            "\n        flags = [\n{}\n        ]",
            default_flags
                .iter()
                .map(|flag| format!("        \"{flag}\""))
                .collect::<Vec<_>>()
                .join(",\n")
        )
    };
    let mut properties = Vec::new();
    for property in &rules.properties {
        match property.name.as_str() {
            "sources" => properties.push(sources_block.clone()),
            "include_dirs" if !include_block.is_empty() => properties.push(include_block.clone()),
            "flags" if !flags.is_empty() => properties.push(flags.clone()),
            "install" if property.automatic => {
                properties.push("        install = true".to_string())
            }
            _ => {}
        }
    }
    format!(
        "project \"{name}\" {{\n    description = \"A project built and automated with Nox.\"\n\n    {target} \"{name}\" {{\n{}\n    }}\n}}\n",
        properties.join("\n")
    )
}
