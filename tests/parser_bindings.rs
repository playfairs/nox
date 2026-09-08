use nox::project::parser::parse_file;
use std::path::Path;

#[test]
fn resolves_project_bindings_in_target_fields() {
    let project = parse_file(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/parser/bindings/nox.build"
    )))
    .expect("binding fixture should parse");

    let target = &project.targets[0];
    assert!(target.sources[0].ends_with(Path::new("main.c")));
    assert!(target.include_dirs[0].ends_with(Path::new("include")));
    assert_eq!(target.flags, ["-Wall", "-Wextra"]);
    assert!(target.install);
}
