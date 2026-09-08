use nox::core::model::TargetKind;
use nox::project::parser::parse_file;
use std::path::Path;

#[test]
fn parses_cxx_executable_target() {
    let project = parse_file(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/parser/cxx/nox.build"
    )))
    .expect("cxx_executable fixture should parse");

    assert_eq!(project.targets[0].kind, TargetKind::CppExecutable);
    assert_eq!(project.targets[0].sources.len(), 1);
}
