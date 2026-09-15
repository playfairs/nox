use super::error::{Error, Result};
use super::language::{Language, ProjectType};
use super::project::ProjectInfo;
use crate::rules::init::InitRules;
use std::io::{self, IsTerminal, Write};
pub fn language(
    project: &ProjectInfo,
    rules: &InitRules,
    requested: Option<&str>,
) -> Result<Language> {
    if let Some(value) = requested {
        return Language::parse_with_rules(value, rules)
            .ok_or_else(|| Error::UnsupportedLanguage(value.to_string()));
    }
    if project.languages.len() == 1 {
        return Ok(*project.languages.first().expect("one language exists"));
    }
    if project.languages.len() > 1 || io::stdin().is_terminal() {
        return prompt_language(project, rules);
    }
    Ok(Language::Rust)
}
fn prompt_language(project: &ProjectInfo, rules: &InitRules) -> Result<Language> {
    let default = project.languages.first().copied().unwrap_or(Language::Rust);
    print!("Language [{}]: ", default.label());
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim();
    if value.is_empty() {
        Ok(default)
    } else {
        Language::parse_with_rules(value, rules)
            .ok_or_else(|| Error::UnsupportedLanguage(value.to_string()))
    }
}
pub fn project_type(project: &ProjectInfo, requested: Option<&str>) -> Result<ProjectType> {
    if let Some(value) = requested {
        return ProjectType::parse(value)
            .ok_or_else(|| Error::Configuration(format!("unsupported project type '{value}'")));
    }
    Ok(project.project_type.unwrap_or(ProjectType::Executable))
}
pub fn infer_type(project: &mut ProjectInfo, rules: &InitRules) {
    if project.project_type.is_none()
        && project.source_files.iter().any(|path| {
            let Some(language) = super::language::Language::from_path(path, rules) else {
                return false;
            };
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            rules
                .language(language)
                .is_some_and(|rule| rule.main_files.iter().any(|name| name == file_name))
        })
    {
        project.project_type = Some(ProjectType::Executable);
    }
}
