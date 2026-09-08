use nox::run::handler_language;
use std::path::Path;

#[test]
fn resolves_supported_file_handlers() {
    for (source, language) in [
        ("Test.fsx", "F#"),
        ("Test.c", "C"),
        ("Test.cpp", "C++"),
        ("Test.d", "D"),
        ("Test.rb", "Ruby"),
        ("Test.py", "Python"),
        ("Test.js", "JavaScript"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/run/handlers")
            .join(source);
        assert_eq!(handler_language(&path).unwrap(), language);
    }
}
