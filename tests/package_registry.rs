use nox::pkgs::{PackageRegistry, PackageSource, PackageSourceKind, PackageRegistryError, PackageRiders};

#[test]
fn parses_supported_install_sources() {
    assert_eq!(
        PackageSource::parse("github:user/repo").unwrap(),
        PackageSource {
            kind: PackageSourceKind::GitHub,
            user: Some("user".to_string()),
            repo: Some("repo".to_string()),
            package: None,
        }
    );
    assert_eq!(
        PackageSource::parse("pkgs:ripnet").unwrap(),
        PackageSource {
            kind: PackageSourceKind::Registry,
            user: None,
            repo: None,
            package: Some("ripnet".to_string()),
        }
    );
}

#[test]
fn registry_parses_valid_package_entries() {
    let json = r#"{
        "packages": [
            {
                "name": "ripnet",
                "description": "Network diagnostics",
                "url": "github:playfairs/ripnet",
                "language": "d",
                "riders": {
                    "commands": ["dmd"]
                }
            },
            {
                "name": "vessel",
                "description": "A vessel for executable payloads.",
                "url": "github:playfairs/vessel",
                "language": "rust",
                "riders": {
                    "commands": ["cargo"]
                }
            }
        ]
    }"#;

    let registry = PackageRegistry::from_str(json).unwrap();
    let package = registry.resolve("ripnet").unwrap();
    assert_eq!(package.name, "ripnet");
    assert_eq!(package.url, "github:playfairs/ripnet");
    assert_eq!(package.riders.commands, vec!["dmd"]);
}

#[test]
fn registry_rejects_duplicate_names() {
    let json = r#"{
        "packages": [
            {"name": "demo", "url": "github:user/demo"},
            {"name": "demo", "url": "github:user/demo-two"}
        ]
    }"#;

    assert!(matches!(
        PackageRegistry::from_str(json),
        Err(PackageRegistryError::DuplicatePackage(_))
    ));
}

#[test]
fn registry_rejects_missing_required_fields() {
    assert!(matches!(
        PackageRegistry::from_str(r#"{"packages":[{"description":"bad"}]}"#),
        Err(PackageRegistryError::MalformedPackage(_))
    ));
}

#[test]
fn registry_rejects_bad_rider_entries() {
    let json = r#"{
        "packages": [
            {"name": "demo", "url": "github:user/demo", "riders": {"commands": [3]}}
        ]
    }"#;

    assert!(matches!(
        PackageRegistry::from_str(json),
        Err(PackageRegistryError::MalformedPackage(_))
    ));
}

#[test]
fn parses_rider_requirements() {
    let riders = PackageRiders::from_commands(vec!["cargo", "dmd"]);
    assert_eq!(riders.commands, vec!["cargo", "dmd"]);
}
