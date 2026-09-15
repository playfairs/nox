use noml::{parse as parse_noml, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct CommandRule {
    pub command: String,
    pub aliases: Vec<String>,
    pub requires_project: bool,
}

#[derive(Clone, Debug)]
struct CommandRules {
    commands: Vec<CommandRule>,
    requires_initialization: Vec<String>,
    accepts_files_as_input: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ArgumentRule {
    pub name: String,
    pub takes_value: bool,
    pub repeatable: bool,
}

#[derive(Clone, Debug)]
pub struct BaseRules {
    pub commands: Vec<CommandRule>,
    pub arguments: Vec<ArgumentRule>,
    pub requires_initialization: Vec<String>,
    pub accepts_files_as_input: Vec<String>,
}

impl BaseRules {
    pub fn load() -> Result<Self, String> {
        let command_rules = parse_commands(include_str!("commands.noml"))?;
        let rules = Self {
            commands: command_rules.commands,
            arguments: parse_arguments(include_str!("args.noml"))?,
            requires_initialization: command_rules.requires_initialization,
            accepts_files_as_input: command_rules.accepts_files_as_input,
        };
        rules.validate()?;
        Ok(rules)
    }

    pub fn command_name(&self, name: &str) -> Option<&str> {
        self.commands.iter().find_map(|rule| {
            (rule.command == name || rule.aliases.iter().any(|alias| alias == name))
                .then_some(rule.command.as_str())
        })
    }

    pub fn command(&self, name: &str) -> Option<&CommandRule> {
        self.commands.iter().find(|rule| rule.command == name)
    }

    pub fn requires_initialization(&self, command: &str) -> bool {
        self.requires_initialization
            .iter()
            .any(|name| name == command)
    }

    pub fn accepts_files_as_input(&self, command: &str) -> bool {
        self.accepts_files_as_input
            .iter()
            .any(|name| name == command)
    }

    fn validate(&self) -> Result<(), String> {
        if self.commands.is_empty() {
            return Err("base command rules cannot be empty".into());
        }
        for (index, rule) in self.commands.iter().enumerate() {
            if rule.command.is_empty() {
                return Err(format!("base command rule {index} has no command"));
            }
            if self
                .commands
                .iter()
                .take(index)
                .any(|other| other.command == rule.command)
            {
                return Err(format!("duplicate base command '{}'", rule.command));
            }
        }
        for rule in &self.arguments {
            if rule.name.is_empty() {
                return Err("base argument rule has no name".into());
            }
        }
        Ok(())
    }
}

fn parse_commands(contents: &str) -> Result<CommandRules, String> {
    let document = parse_noml(contents).map_err(|error| format!("invalid base command rules: {error}"))?;

    let ruleset = match document {
        Value::Ruleset(ruleset) => ruleset,
        Value::Array(entries) => {
            let mut command_entries = Vec::new();
            for entry in entries {
                let Value::Object(fields) = entry else {
                    return Err("invalid base command rules: expected an object entry".into());
                };
                let command = field_string(&fields, "command")?;
                let aliases = string_list(&fields, "aliases")?;
                let rules = match fields.get("rules") {
                    Some(Value::Object(rules)) => rules,
                    Some(other) => return Err(format!("command '{command}' rules must be an object, got {other:?}")),
                    None => &BTreeMap::new(),
                };
                let requires_project = bool_option(rules, "requires_project").unwrap_or(false);
                let requires_init = bool_option(rules, "requires_initialization").unwrap_or(false);
                let accepts_input = bool_option(rules, "accepts_files_as_input").unwrap_or(false);
                command_entries.push((command, aliases, requires_project, requires_init, accepts_input));
            }
            let mut commands = Vec::new();
            let mut requires_initialization = Vec::new();
            let mut accepts_files_as_input = Vec::new();
            for (command, aliases, requires_project, requires_init, accepts_input) in command_entries {
                commands.push(CommandRule { command: command.clone(), aliases, requires_project });
                if requires_init { requires_initialization.push(command.clone()); }
                if accepts_input { accepts_files_as_input.push(command); }
            }
            return Ok(CommandRules { commands, requires_initialization, accepts_files_as_input });
        }
        other => return Err(format!("invalid base command rules: expected a ruleset or array, got {other:?}")),
    };

    let mut commands = Vec::new();
    let mut requires_initialization = Vec::new();
    let mut accepts_files_as_input = Vec::new();

    for entry in ruleset.entries {
        if entry.type_name != "command" {
            continue;
        }

        let command = entry.name;
        let aliases = match entry.properties.get("aliases") {
            Some(Value::Array(values)) => values.iter().map(|value| match value {
                Value::String(text) => Ok(text.clone()),
                _ => Err(format!("command '{command}' aliases must be strings")),
            }).collect::<Result<Vec<_>, String>>()?,
            Some(_) => return Err(format!("command '{command}' aliases must be an array")),
            None => Vec::new(),
        };

        let rules = match entry.properties.get("rules") {
            Some(Value::Object(rules)) => rules,
            Some(other) => return Err(format!("command '{command}' rules must be an object, got {other:?}")),
            None => &BTreeMap::new(),
        };

        let requires_project = bool_option(rules, "requires_project").unwrap_or(false);
        let requires_init = bool_option(rules, "requires_initialization").unwrap_or(false);
        let accepts_input = bool_option(rules, "accepts_files_as_input").unwrap_or(false);

        commands.push(CommandRule {
            command: command.clone(),
            aliases,
            requires_project,
        });

        if requires_init {
            requires_initialization.push(command.clone());
        }
        if accepts_input {
            accepts_files_as_input.push(command);
        }
    }

    Ok(CommandRules {
        commands,
        requires_initialization,
        accepts_files_as_input,
    })
}

fn parse_arguments(contents: &str) -> Result<Vec<ArgumentRule>, String> {
    let document = parse_noml(contents).map_err(|error| format!("invalid base argument rules: {error}"))?;
    match document {
        Value::Ruleset(ruleset) => {
            let mut arguments = Vec::new();
            for entry in ruleset.entries {
                if entry.type_name != "argument" {
                    continue;
                }
                let takes_value = bool_field(&entry.properties, "takes_value").unwrap_or(false);
                let repeatable = bool_field(&entry.properties, "repeatable").unwrap_or(false);
                arguments.push(ArgumentRule {
                    name: entry.name,
                    takes_value,
                    repeatable,
                });
            }
            Ok(arguments)
        }
        Value::Array(entries) => {
            let mut arguments = Vec::new();
            for entry in entries {
                let Value::Object(fields) = entry else {
                    return Err("invalid base argument rules: expected an object entry".into());
                };
                arguments.push(ArgumentRule {
                    name: field_string(&fields, "argument")?,
                    takes_value: bool_field(&fields, "takes_value").unwrap_or(false),
                    repeatable: bool_field(&fields, "repeatable").unwrap_or(false),
                });
            }
            Ok(arguments)
        }
        other => Err(format!("invalid base argument rules: expected a ruleset or array, got {other:?}")),
    }
}

fn field_string(fields: &BTreeMap<String, Value>, name: &str) -> Result<String, String> {
    match fields.get(name) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(Value::Integer(value)) => Ok(value.to_string()),
        Some(Value::Boolean(value)) => Ok(value.to_string()),
        Some(Value::Float(value)) => Ok(value.to_string()),
        Some(other) => Err(format!("field '{name}' must be a scalar string, got {other:?}")),
        None => Err(format!("missing field '{name}'")),
    }
}

fn bool_option(fields: &BTreeMap<String, Value>, name: &str) -> Option<bool> {
    fields.get(name).and_then(|value| match value {
        Value::Boolean(value) => Some(*value),
        _ => None,
    })
}

fn bool_field(fields: &BTreeMap<String, Value>, name: &str) -> Result<bool, String> {
    match fields.get(name) {
        Some(Value::Boolean(value)) => Ok(*value),
        Some(other) => Err(format!("field '{name}' must be a boolean, got {other:?}")),
        None => Ok(false),
    }
}

fn string_list(fields: &BTreeMap<String, Value>, name: &str) -> Result<Vec<String>, String> {
    match fields.get(name) {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(text) => Ok(text.clone()),
                _ => Err(format!("field '{name}' must contain only strings")),
            })
            .collect(),
        Some(_) => Err(format!("field '{name}' must be an array of strings")),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::BaseRules;

    #[test]
    fn loads_command_initialization_rules() {
        let rules = BaseRules::load().expect("embedded base rules should load");
        assert_eq!(rules.command_name("b"), Some("build"));
        assert!(!rules.command("version").unwrap().requires_project);
        assert!(!rules.requires_initialization("setup"));
        assert!(rules.accepts_files_as_input("run"));
        assert!(rules.requires_initialization("run"));
    }
}
