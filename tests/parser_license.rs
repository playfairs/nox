use nox::project::parser::parse;
use std::fs;

#[test]
fn recognizes_license_file_as_spdx_identifier() {
    let root = std::env::temp_dir().join(format!("nox-license-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("UNLICENSE"),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/UNLICENSE")),
    )
    .unwrap();

    let project = parse(
        "project \"fixture\" { license = file(\"./UNLICENSE\") executable \"fixture\" { sources = [\"main.c\"] } }",
        &root,
    )
    .expect("license file should be recognized");
    assert_eq!(project.license, "Unlicense");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_unrecognized_license_file() {
    let root = std::env::temp_dir().join(format!("nox-license-invalid-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("LICENSE"), "not a real license").unwrap();

    let error = parse(
        "project \"fixture\" { license = file(\"LICENSE\") executable \"fixture\" { sources = [\"main.c\"] } }",
        &root,
    )
    .expect_err("fake license should be rejected");
    assert!(error.to_string().contains("could not identify"));
    fs::remove_dir_all(root).unwrap();
}
