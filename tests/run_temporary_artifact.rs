use nox::run::TemporaryArtifact;
use std::path::Path;

#[test]
fn temporary_artifacts_live_outside_the_source_tree_and_clean_up() {
    let source = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/run/c/Test.c"
    ));
    let artifact = TemporaryArtifact::new(source).expect("temp artifact");
    let directory = artifact.path().parent().expect("artifact directory");
    assert!(!artifact.path().starts_with(source.parent().unwrap()));
    assert!(directory.exists());
    let directory = directory.to_path_buf();
    drop(artifact);
    assert!(!directory.exists());
}
