use nox::core::model::{TargetKind, TargetLanguage};
use nox::project::parser::parse;
use nox::project::parser::{parse_file, parse_projects};
use std::path::Path;

#[test]
fn parses_cpp_executable_target() {
    let project = parse_file(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/parser/cxx/nox.build"
    )))
    .expect("executable.cpp fixture should parse");

    assert_eq!(
        project.targets[0].kind,
        TargetKind::Executable {
            language: Some(TargetLanguage::Cpp)
        }
    );
    assert_eq!(project.targets[0].sources.len(), 1);
}

#[test]
fn parses_rust_qualified_executable_target() {
    let project = parse(
        "project \"nox\" { executable.rust \"nox\" { sources = [\"src/main.rs\"] } }",
        Path::new("."),
    )
    .expect("executable.rust fixture should parse");

    assert_eq!(
        project.targets[0].kind,
        TargetKind::Executable {
            language: Some(TargetLanguage::Rust)
        }
    );
}

#[test]
fn parses_multiple_projects() {
    let projects = parse_projects(
        "project \"first\" { executable.rust \"one\" { sources = [\"one.rs\"] } } project \"second\" { executable.rust \"two\" { sources = [\"two.rs\"] } }",
        Path::new("."),
    )
    .expect("multiple projects should parse");

    assert_eq!(
        projects
            .iter()
            .map(|project| project.name.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
}

#[test]
fn parses_project_metadata_fields() {
    let project = parse(
        "project \"nox\" { version = \"1.2.3\" description = \"demo\" repository = \"https://example.com/repo\" website = \"https://example.com\" authors = [\"alice\", \"bob\"] maintainers = [\"carol <carol@example.com>\"] executable.rust \"app\" { sources = [\"src/main.rs\"] } }",
        Path::new("."),
    )
    .expect("project metadata should parse");

    assert_eq!(project.version.as_deref(), Some("1.2.3"));
    assert_eq!(project.repository.as_deref(), Some("https://example.com/repo"));
    assert_eq!(project.website.as_deref(), Some("https://example.com"));
    assert_eq!(project.authors, vec!["alice", "bob"]);
    assert_eq!(project.maintainers, vec!["carol <carol@example.com>"]);
}

#[test]
fn parses_project_extra_environment() {
    let project = parse(
        "project \"demo\" { extra.env { RUST_BACKTRACE = \"full\" CARGO_TERM_COLOR = \"always\" } executable \"demo\" { sources = [\"src/main.rs\"] } }",
        Path::new("."),
    )
    .expect("extra.env should parse");

    assert_eq!(project.extra_env.get("RUST_BACKTRACE"), Some(&"full".to_string()));
    assert_eq!(
        project.extra_env.get("CARGO_TERM_COLOR"),
        Some(&"always".to_string())
    );
}

#[test]
fn rejects_unsupported_qualified_executable_language() {
    let error = parse(
        "project \"example\" { executable.unknown \"app\" { sources = [\"main.c\"] } }",
        Path::new("."),
    )
    .expect_err("unsupported executable language should fail");

    assert!(
        error
            .to_string()
            .contains("unsupported executable language")
    );
}
