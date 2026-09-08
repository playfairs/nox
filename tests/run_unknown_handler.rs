use nox::run::handler_language;
use std::path::Path;

#[test]
fn rejects_unknown_file_handlers() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/run/handlers/Test.unknown");
    assert!(handler_language(&path).is_err());
}
