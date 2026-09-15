use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub enum Language {
    Rust,
    Haskell,
    C,
    Cpp,
    D,
    Swift,
    FSharp,
    JavaScript,
    TypeScript,
    Python,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum ProjectType {
    Executable,
    Library,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LanguageRule {
    pub language: Language,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub extensions: Vec<String>,
    pub header_extensions: Vec<String>,
    pub related_files: Vec<String>,
    pub target_types: Vec<String>,
    pub main_files: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LayoutRule {
    pub name: String,
    pub directory: String,
    pub priority: i32,
    pub role: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileRule {
    pub name: String,
    pub languages: Vec<Language>,
    pub build_system: Option<String>,
    pub project_name: Option<String>,
    pub project_type: Option<ProjectType>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IgnoreRule {
    pub path: String,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ConventionRule {
    pub name: String,
    pub values: Vec<String>,
    pub priority: i32,
    pub role: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TargetRule {
    pub language: Language,
    pub project_type: ProjectType,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PropertyRule {
    pub name: String,
    pub order: u32,
    pub required: bool,
    pub automatic: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FlagRule {
    pub name: String,
    pub values: Vec<String>,
    pub languages: Vec<Language>,
    pub order: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TemplateRule {
    pub name: String,
    pub path: String,
    pub languages: Vec<Language>,
    pub optional: bool,
}

#[derive(Clone, Debug)]
pub struct InitRules {
    pub languages: Vec<LanguageRule>,
    pub layouts: Vec<LayoutRule>,
    pub files: Vec<FileRule>,
    pub ignores: Vec<IgnoreRule>,
    pub conventions: Vec<ConventionRule>,
    pub targets: Vec<TargetRule>,
    pub properties: Vec<PropertyRule>,
    pub flags: Vec<FlagRule>,
    pub templates: Vec<TemplateRule>,
}

impl InitRules {
    pub fn load() -> Result<Self, String> {
        let rules = Self {
            languages: parse(include_str!("languages.ron"), "languages")?,
            layouts: parse(include_str!("layouts.ron"), "layouts")?,
            files: parse(include_str!("files.ron"), "files")?,
            ignores: parse(include_str!("ignores.ron"), "ignores")?,
            conventions: parse(include_str!("conventions.ron"), "conventions")?,
            targets: parse(include_str!("targets.ron"), "targets")?,
            properties: parse(include_str!("properties.ron"), "properties")?,
            flags: parse(include_str!("flags.ron"), "flags")?,
            templates: parse(include_str!("templates.ron"), "templates")?,
        };
        rules.validate()?;
        Ok(rules)
    }

    fn validate(&self) -> Result<(), String> {
        if self.languages.is_empty() {
            return Err("init languages cannot be empty".into());
        }
        let languages = self
            .languages
            .iter()
            .map(|rule| rule.language)
            .collect::<BTreeSet<_>>();
        for rule in &self.targets {
            if !languages.contains(&rule.language) {
                return Err(format!(
                    "target rule references unknown language {:?}",
                    rule.language
                ));
            }
        }
        for rule in &self.flags {
            if rule.values.is_empty() {
                return Err(format!("flag rule '{}' has no values", rule.name));
            }
        }
        if self
            .properties
            .windows(2)
            .any(|rules| rules[0].order >= rules[1].order)
        {
            return Err("init properties must be ordered by increasing order".into());
        }
        Ok(())
    }

    pub fn language(&self, language: Language) -> Option<&LanguageRule> {
        self.languages.iter().find(|rule| rule.language == language)
    }
    pub fn target(&self, language: Language, project_type: ProjectType) -> Option<&TargetRule> {
        self.targets
            .iter()
            .find(|rule| rule.language == language && rule.project_type == project_type)
    }
    pub fn template(&self, name: &str, language: Language) -> Option<&TemplateRule> {
        self.templates.iter().find(|rule| {
            rule.name == name && (rule.languages.is_empty() || rule.languages.contains(&language))
        })
    }
}

fn parse<T: for<'de> Deserialize<'de>>(contents: &str, name: &str) -> Result<T, String> {
    ron::from_str(contents).map_err(|error| format!("invalid init {name} rules: {error}"))
}
