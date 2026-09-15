use crate::rules::init::InitRules;
use std::path::Path;

pub use crate::rules::init::{Language, ProjectType};

impl Language {
    pub fn parse(value: &str) -> Option<Self> {
        Self::parse_with_rules(value, embedded_rules())
    }
    pub fn parse_with_rules(value: &str, rules: &InitRules) -> Option<Self> {
        let value = value.to_ascii_lowercase();
        rules
            .languages
            .iter()
            .find(|rule| {
                rule.display_name.to_ascii_lowercase() == value
                    || rule
                        .aliases
                        .iter()
                        .any(|alias| alias.to_ascii_lowercase() == value)
            })
            .map(|rule| rule.language)
    }
    pub fn label(self) -> String {
        static RULES: std::sync::OnceLock<InitRules> = std::sync::OnceLock::new();
        RULES.get_or_init(|| InitRules::load().expect("embedded init rules are valid")).language(self).map(|rule| rule.display_name.clone()).unwrap_or_else(|| "unknown language".to_string())
    }
    pub fn from_path(path: &Path, rules: &InitRules) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        rules.languages.iter().find(|rule| rule.extensions.iter().any(|value| value == extension)).map(|rule| rule.language)
    }
    pub fn from_extension(extension: &str) -> Option<Self> {
        Self::from_path(Path::new(&format!("source.{extension}")), embedded_rules())
    }
    pub fn is_header(path: &Path, rules: &InitRules) -> bool {
        path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| rules.languages.iter().any(|rule| rule.header_extensions.iter().any(|value| value == extension)))
    }
}

impl ProjectType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "executable" | "exe" | "binary" | "bin" => Some(Self::Executable),
            "library" | "lib" => Some(Self::Library),
            _ => None,
        }
    }
}
pub fn is_test_path(path: &Path) -> bool {
    is_test_path_with_rules(path, embedded_rules())
}
pub fn is_test_path_with_rules(path: &Path, rules: &InitRules) -> bool {
    let names = rules.conventions.iter().find(|rule| rule.name == "test_directories").map(|rule| &rule.values);
    path.components().any(|component| names.is_some_and(|values| values.iter().any(|value| component.as_os_str() == std::ffi::OsStr::new(value))))
}

fn embedded_rules() -> &'static InitRules {
    static RULES: std::sync::OnceLock<InitRules> = std::sync::OnceLock::new();
    RULES.get_or_init(|| InitRules::load().expect("embedded init rules are valid"))
}
