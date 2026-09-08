use nox::project::parser::parse_file;
use std::path::Path;

#[test]
fn rejects_duplicate_bindings() {
    let error = parse_file(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/parser/duplicate-bindings/nox.build"
    )))
    .expect_err("duplicate bindings should fail");

    assert!(error.to_string().contains("declared more than once"));
}
