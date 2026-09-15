use noml::{parse as parse_noml, Value};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

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
            languages: parse_list_of_objects(include_str!("languages.noml"), "languages", parse_language_rule)?,
            layouts: parse_list_of_objects(include_str!("layouts.noml"), "layouts", parse_layout_rule)?,
            files: parse_list_of_objects(include_str!("files.noml"), "files", parse_file_rule)?,
            ignores: parse_list_of_objects(include_str!("ignores.noml"), "ignores", parse_ignore_rule)?,
            conventions: parse_list_of_objects(include_str!("conventions.noml"), "conventions", parse_convention_rule)?,
            targets: parse_list_of_objects(include_str!("targets.noml"), "targets", parse_target_rule)?,
            properties: parse_list_of_objects(include_str!("properties.noml"), "properties", parse_property_rule)?,
            flags: parse_list_of_objects(include_str!("flags.noml"), "flags", parse_flag_rule)?,
            templates: parse_list_of_objects(include_str!("templates.noml"), "templates", parse_template_rule)?,
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

fn parse_list_of_objects<T>(contents: &str, name: &str, converter: fn(&BTreeMap<String, Value>) -> Result<T, String>) -> Result<Vec<T>, String> {
    let document = parse_noml(contents).map_err(|error| format!("invalid init {name} rules: {error}"))?;
    let entries = match document {
        Value::Ruleset(ruleset) => ruleset.entries,
        Value::Array(entries) => entries
            .into_iter()
            .map(|entry| match entry {
                Value::Object(fields) => Ok({
                    let mut entry_map = BTreeMap::new();
                    for (key, value) in fields {
                        entry_map.insert(key, value);
                    }
                    noml::Entry {
                        type_name: name.trim_end_matches('s').to_string(),
                        name: entry_map
                            .get("name")
                            .and_then(|value| match value {
                                Value::String(text) => Some(text.clone()),
                                _ => None,
                            })
                            .unwrap_or_default(),
                        properties: entry_map,
                        extends: None,
                    }
                }),
                other => Err(format!("invalid init {name} rules: expected an object entry, got {other:?}")),
            })
            .collect::<Result<Vec<_>, String>>()?,
        other => return Err(format!("invalid init {name} rules: expected a ruleset or array, got {other:?}")),
    };

    entries
        .iter()
        .map(|entry| {
            let mut fields = entry.properties.clone();
            if !fields.contains_key("name") && !entry.name.is_empty() {
                fields.insert("name".to_string(), Value::String(entry.name.clone()));
            }
            if entry.type_name == "language" && !fields.contains_key("language") && !entry.name.is_empty() {
                fields.insert("language".to_string(), Value::String(entry.name.clone()));
            }
            if entry.type_name == "argument" && !fields.contains_key("argument") && !entry.name.is_empty() {
                fields.insert("argument".to_string(), Value::String(entry.name.clone()));
            }
            if entry.type_name == "ignore" && !fields.contains_key("path") && !entry.name.is_empty() {
                fields.insert("path".to_string(), Value::String(entry.name.clone()));
            }
            if entry.type_name == "target" && !fields.contains_key("target") && !entry.name.is_empty() {
                fields.insert("target".to_string(), Value::String(entry.name.clone()));
            }
            converter(&fields)
        })
        .collect::<Result<Vec<_>, String>>()
}

fn parse_language_rule(fields: &BTreeMap<String, Value>) -> Result<LanguageRule, String> {
    Ok(LanguageRule {
        language: parse_language(&field_string(fields, "language")?)?,
        display_name: field_string(fields, "display_name")?,
        aliases: string_list(fields, "aliases")?,
        extensions: string_list(fields, "extensions")?,
        header_extensions: string_list(fields, "header_extensions")?,
        related_files: string_list(fields, "related_files")?,
        target_types: string_list(fields, "target_types")?,
        main_files: string_list(fields, "main_files")?,
    })
}

fn parse_layout_rule(fields: &BTreeMap<String, Value>) -> Result<LayoutRule, String> {
    Ok(LayoutRule {
        name: field_string(fields, "name")?,
        directory: field_string(fields, "directory")?,
        priority: field_integer(fields, "priority")? as i32,
        role: field_string(fields, "role")?,
    })
}

fn parse_file_rule(fields: &BTreeMap<String, Value>) -> Result<FileRule, String> {
    Ok(FileRule {
        name: field_string(fields, "name")?,
        languages: string_list(fields, "languages")?
            .into_iter()
            .map(|value| parse_language(&value))
            .collect::<Result<Vec<_>, String>>()?,
        build_system: optional_string(fields, "build_system")?,
        project_name: optional_string(fields, "project_name")?,
        project_type: optional_project_type(fields, "project_type")?,
    })
}

fn parse_ignore_rule(fields: &BTreeMap<String, Value>) -> Result<IgnoreRule, String> {
    Ok(IgnoreRule {
        path: field_string(fields, "path")?,
        kind: field_string(fields, "kind")?,
    })
}

fn parse_convention_rule(fields: &BTreeMap<String, Value>) -> Result<ConventionRule, String> {
    Ok(ConventionRule {
        name: field_string(fields, "name")?,
        values: string_list(fields, "values")?,
        priority: field_integer(fields, "priority")? as i32,
        role: field_string(fields, "role")?,
    })
}

fn parse_target_rule(fields: &BTreeMap<String, Value>) -> Result<TargetRule, String> {
    Ok(TargetRule {
        language: parse_language(&field_string(fields, "language")?)?,
        project_type: parse_project_type(&field_string(fields, "project_type")?)?,
        target: field_string(fields, "target")?,
    })
}

fn parse_property_rule(fields: &BTreeMap<String, Value>) -> Result<PropertyRule, String> {
    Ok(PropertyRule {
        name: field_string(fields, "name")?,
        order: field_integer(fields, "order")? as u32,
        required: field_bool(fields, "required")?,
        automatic: field_bool(fields, "automatic")?,
    })
}

fn parse_flag_rule(fields: &BTreeMap<String, Value>) -> Result<FlagRule, String> {
    Ok(FlagRule {
        name: field_string(fields, "name")?,
        values: string_list(fields, "values")?,
        languages: string_list(fields, "languages")?
            .into_iter()
            .map(|value| parse_language(&value))
            .collect::<Result<Vec<_>, String>>()?,
        order: field_integer(fields, "order")? as u32,
    })
}

fn parse_template_rule(fields: &BTreeMap<String, Value>) -> Result<TemplateRule, String> {
    Ok(TemplateRule {
        name: field_string(fields, "name")?,
        path: field_string(fields, "path")?,
        languages: string_list(fields, "languages")?
            .into_iter()
            .map(|value| parse_language(&value))
            .collect::<Result<Vec<_>, String>>()?,
        optional: field_bool(fields, "optional")?,
    })
}

fn parse_language(value: &str) -> Result<Language, String> {
    match value.to_ascii_lowercase().as_str() {
        "rust" => Ok(Language::Rust),
        "haskell" => Ok(Language::Haskell),
        "c" => Ok(Language::C),
        "cpp" | "c++" | "cxx" => Ok(Language::Cpp),
        "d" => Ok(Language::D),
        "swift" => Ok(Language::Swift),
        "fsharp" | "f#" => Ok(Language::FSharp),
        "javascript" | "js" => Ok(Language::JavaScript),
        "typescript" | "ts" => Ok(Language::TypeScript),
        "python" | "py" => Ok(Language::Python),
        "unknown" => Ok(Language::Unknown),
        other => Err(format!("unknown language '{other}'")),
    }
}

fn parse_project_type(value: &str) -> Result<ProjectType, String> {
    match value.to_ascii_lowercase().as_str() {
        "executable" => Ok(ProjectType::Executable),
        "library" => Ok(ProjectType::Library),
        other => Err(format!("unknown project type '{other}'")),
    }
}

fn field_string(fields: &BTreeMap<String, Value>, name: &str) -> Result<String, String> {
    let value = fields.get(name).ok_or_else(|| format!("missing field '{name}'"))?;
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Integer(value) => Ok(value.to_string()),
        Value::Boolean(value) => Ok(value.to_string()),
        Value::Float(value) => Ok(value.to_string()),
        other => Err(format!("field '{name}' must be a scalar string, got {other:?}")),
    }
}

fn optional_string(fields: &BTreeMap<String, Value>, name: &str) -> Result<Option<String>, String> {
    match fields.get(name) {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(_) => Ok(Some(field_string(fields, name)?)),
    }
}

fn optional_project_type(fields: &BTreeMap<String, Value>, name: &str) -> Result<Option<ProjectType>, String> {
    match fields.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match value {
            Value::String(value) => Ok(Some(parse_project_type(value)?)),
            _ => Err(format!("field '{name}' must be a string or null")),
        },
    }
}

fn field_bool(fields: &BTreeMap<String, Value>, name: &str) -> Result<bool, String> {
    match fields.get(name).ok_or_else(|| format!("missing field '{name}'"))? {
        Value::Boolean(value) => Ok(*value),
        _ => Err(format!("field '{name}' must be a boolean")),
    }
}

fn field_integer(fields: &BTreeMap<String, Value>, name: &str) -> Result<i64, String> {
    match fields.get(name).ok_or_else(|| format!("missing field '{name}'"))? {
        Value::Integer(value) => Ok(*value),
        _ => Err(format!("field '{name}' must be an integer")),
    }
}

fn string_list(fields: &BTreeMap<String, Value>, name: &str) -> Result<Vec<String>, String> {
    match fields.get(name).ok_or_else(|| format!("missing field '{name}'"))? {
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::String(text) => Ok(text.clone()),
                _ => Err(format!("field '{name}' must contain only strings")),
            })
            .collect(),
        other => Err(format!("field '{name}' must be an array, got {other:?}")),
    }
}
