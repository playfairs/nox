use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CommandRule {
    pub command: String,
    pub aliases: Vec<String>,
    pub requires_project: bool,
    pub requires_initialization: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ArgumentRule {
    pub name: String,
    pub takes_value: bool,
    pub repeatable: bool,
}

#[derive(Clone, Debug)]
pub struct BaseRules {
    pub commands: Vec<CommandRule>,
    pub arguments: Vec<ArgumentRule>,
}

impl BaseRules {
    pub fn load() -> Result<Self, String> {
        let rules = Self {
            commands: parse(include_str!("initialized.ron"), "initialized")?,
            arguments: parse(include_str!("args.ron"), "arguments")?,
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

fn parse<T: for<'de> Deserialize<'de>>(contents: &str, name: &str) -> Result<T, String> {
    ron::from_str(contents).map_err(|error| format!("invalid base {name} rules: {error}"))
}

#[cfg(test)]
mod tests {
    use super::BaseRules;

    #[test]
    fn loads_command_initialization_rules() {
        let rules = BaseRules::load().expect("embedded base rules should load");
        assert_eq!(rules.command_name("b"), Some("build"));
        assert!(rules.command("build").unwrap().requires_initialization);
        assert!(!rules.command("version").unwrap().requires_project);
        assert!(!rules.command("setup").unwrap().requires_initialization);
    }
}