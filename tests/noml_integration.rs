use noml::{parse, serialize, Value};

#[test]
fn parses_basic_noml_values_and_rulesets() {
    let value = parse(
        r#"
        ruleset "commands" {
            command: help {
                description: ["Display help information."]
                aliases: []
                rules: {
                    requires_project: false,
                    requires_initialization: false,
                    accepts_files_as_input: false
                }
            }
        }
        "#,
    )
    .expect("ruleset should parse");

    match value {
        Value::Ruleset(ruleset) => {
            assert_eq!(ruleset.name, "commands");
            assert_eq!(ruleset.entries.len(), 1);
            assert_eq!(ruleset.entries[0].type_name, "command");
            assert_eq!(ruleset.entries[0].name, "help");
            assert_eq!(
                ruleset.entries[0].properties["description"],
                Value::Array(vec![Value::String("Display help information.".to_string())])
            );
        }
        _ => panic!("expected a ruleset"),
    }
}

#[test]
fn serializes_parsed_noml_without_losing_structure() {
    let parsed = parse(
        r#"
        {
            name: "demo"
            enabled: true
            numbers: [1, 2, 3]
            nested: {
                flag: false
            }
        }
        "#,
    )
    .expect("object should parse");

    let text = serialize(&parsed);
    assert!(text.contains("name: demo"));
    assert!(text.contains("enabled: true"));
    assert!(text.contains("numbers: [1, 2, 3]"));
    assert!(text.contains("nested: {"));
}

#[test]
fn loads_the_nox_command_rules_noml_file() {
    let parsed = parse(include_str!("../src/rules/base/commands.noml")).expect("Nox commands file should parse");
    match parsed {
        Value::Ruleset(ruleset) => {
            assert_eq!(ruleset.name, "commands");
            assert!(!ruleset.entries.is_empty());
            assert!(ruleset.entries.iter().any(|entry| entry.type_name == "command" && entry.name == "help"));
            assert!(ruleset.entries.iter().any(|entry| entry.type_name == "command" && entry.name == "task"));
        }
        _ => panic!("expected a ruleset"),
    }
}
