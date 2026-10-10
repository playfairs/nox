use crate::core::error::{Error, Result};
use crate::core::output;
use crate::pkgs::database::{InstallMethod, InstallOrigin, InstalledPackageDatabase};
use crate::pkgs::fetch::{fetch_github_source, fetch_registry_index};
use crate::pkgs::{Package, PackageSource, PackageSourceKind};
use crate::project::parser;
use crate::toolchain::detection;
use std::path::Path;

pub fn run_package_command(arguments: &[String]) -> Result<()> {
    match arguments.first().map(String::as_str) {
        None | Some("list") => list_installed_packages(),
        Some("search") => search_registry(arguments.get(1).map(String::as_str).unwrap_or("")),
        Some("info") => {
            let name = arguments
                .get(1)
                .ok_or_else(|| Error::Config("package info requires a name".to_string()))?;
            show_installed_package(name)
        }
        Some(name) => show_installed_package(name),
    }
}

fn list_installed_packages() -> Result<()> {
    let database = InstalledPackageDatabase::load()?;
    let mut seen = std::collections::HashSet::new();
    for package in &database.packages {
        if !seen.insert(package.name.to_ascii_lowercase()) {
            continue;
        }
        let methods = database
            .find(&package.name)
            .iter()
            .map(|package| package.method.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        let description = if package.description.is_empty() {
            String::new()
        } else {
            format!(" - {}", package.description)
        };
        output::item(format!(
            "{}{} (installed via {methods})",
            package.name, description
        ));
    }
    if database.packages.is_empty() {
        output::item("No packages are installed.");
    }
    Ok(())
}

fn search_registry(query: &str) -> Result<()> {
    let registry = fetch_registry_index()?;
    let matches = registry.search(query);
    if matches.is_empty() {
        return Err(Error::Config(format!(
            "no packages matched '{query}' in the Nox package registry"
        )));
    }
    for package in matches {
        output::item(format!(
            "{}{}",
            package.name,
            package
                .description
                .as_ref()
                .map(|description| format!(" - {description}"))
                .unwrap_or_default()
        ));
    }
    Ok(())
}

fn show_installed_package(name: &str) -> Result<()> {
    let database = InstalledPackageDatabase::load()?;
    let installations = database.find(name);
    let Some(package) = installations.last() else {
        return Err(Error::Config(format!("package '{name}' is not installed")));
    };

    println!("{}", package.name);
    output::key_value("Description", package.description.as_str());
    output::key_value("License", package.license.as_str());
    output::key_value("Language", display_values(&package.language));
    output::key_value("Riders", display_values(&package.riders));
    output::key_value(
        "Required commands",
        display_values(&package.required_commands),
    );
    for installation in installations {
        output::key_value("Installed via", installation.method.as_str());
        if let Some(url) = &installation.url {
            output::key_value("GitHub URL", url.as_str());
        }
        if let Some(path) = &installation.path {
            output::key_value("Local path", path.display());
        }
        for destination in &installation.installed_to {
            output::key_value("Installed to", destination.display());
        }
    }
    Ok(())
}

fn display_values(values: &[String]) -> String {
    if values.is_empty() {
        "unknown".to_string()
    } else {
        values.join(", ")
    }
}

pub fn install_from_reference(
    reference: &str,
    prefix: &Path,
    configuration: &str,
    jobs: usize,
) -> Result<()> {
    let requested_source =
        PackageSource::parse(reference).map_err(|error| Error::Config(error.to_string()))?;
    let result = match requested_source.kind {
        PackageSourceKind::GitHub => install_github_source(
            &requested_source,
            prefix,
            configuration,
            jobs,
            None,
            InstallOrigin {
                method: InstallMethod::GitHub,
                url: Some(reference.to_string()),
                path: None,
            },
        ),
        PackageSourceKind::Registry => {
            let package_name = requested_source.package.as_deref().ok_or_else(|| {
                Error::Config("package registry sources must include a package name".to_string())
            })?;
            let registry = fetch_registry_index()?;
            let package = registry
                .resolve(package_name)
                .map_err(|error| Error::Config(error.to_string()))?;
            validate_package_riders(package)?;
            let source = PackageSource::parse(&package.url)
                .map_err(|error| Error::Config(error.to_string()))?;
            install_github_source(
                &source,
                prefix,
                configuration,
                jobs,
                Some(package),
                InstallOrigin {
                    method: InstallMethod::Registry,
                    url: Some(package.url.clone()),
                    path: None,
                },
            )
        }
    };
    result.map_err(|error| add_missing_rider_guidance(error, reference))
}

fn add_missing_rider_guidance(error: Error, reference: &str) -> Error {
    match error {
        Error::Config(message)
            if message.contains("Rider")
                && (message.contains("not found") || message.contains("was not found")) =>
        {
            let requirement = missing_rider_requirement(&message).unwrap_or_else(|| {
                "Check the package's Rider requirements and project language.".to_string()
            });
            Error::Config(format!(
                "{message}\n{requirement}\nNox cannot build '{reference}' because its project requires a language Rider/toolchain that is not available. Remote package and GitHub installation is a new feature, and Nox does not yet have a widely supported system for downloading or installing toolchains automatically. Install the required compiler or Rider yourself, make it available on PATH, and retry."
            ))
        }
        error => error,
    }
}

fn missing_rider_requirement(message: &str) -> Option<String> {
    if let Some(requirement) = message
        .split_once(" requires Rider \"")
        .and_then(|(_, remainder)| remainder.split_once('"').map(|(command, _)| command))
    {
        return Some(format!(
            "This package explicitly requires the '{requirement}' command on PATH."
        ));
    }

    let rider = message
        .strip_prefix("toolchain for ")?
        .strip_suffix(" Rider was not found")?;
    let compilers = match rider {
        "D" => "ldc2, dmd, or gdc",
        "Go" => "go",
        "Java" => "javac",
        "CSharp" => "dotnet, csc, or mcs",
        "QSharp" => "dotnet",
        "Swift" => "swiftc",
        "Zig" => "zig",
        "Python" => "python3 or python",
        "JavaScript" => "node",
        "TypeScript" => "tsc",
        "Kotlin" => "kotlinc",
        "Haskell" => "ghc",
        _ => return None,
    };
    Some(format!(
        "The {rider} Rider needs one of these commands on PATH: {compilers}."
    ))
}

fn install_github_source(
    source: &PackageSource,
    prefix: &Path,
    configuration: &str,
    jobs: usize,
    package: Option<&Package>,
    origin: InstallOrigin,
) -> Result<()> {
    let repository = fetch_github_source(source)?;
    let root = repository.root();
    validate_source_root(root, source, package)?;
    crate::cli::install_project_with_origin(
        root,
        &root.join("build"),
        configuration,
        jobs,
        prefix,
        None,
        origin,
        package,
    )
}

fn validate_package_riders(package: &Package) -> Result<()> {
    for requirement in &package.riders.commands {
        let requirement = requirement.trim();
        if requirement.is_empty() {
            continue;
        }
        if detection::require(
            &[requirement],
            &format!(
                "Package \"{}\" requires Rider \"{}\"",
                package.name, requirement
            ),
        )
        .is_err()
        {
            return Err(Error::Config(format!(
                "Package \"{}\" requires Rider \"{}\", but it was not found.",
                package.name, requirement
            )));
        }
    }
    Ok(())
}

fn validate_source_root(
    root: &Path,
    source: &PackageSource,
    package: Option<&Package>,
) -> Result<()> {
    let build_file = root.join("nox.build");
    if !build_file.is_file() {
        return Err(Error::Config(format!(
            "source '{}' does not contain a valid nox.build file",
            source_name(source)
        )));
    }
    let project = parser::parse_file(&build_file).map_err(|error| {
        Error::Config(format!(
            "source '{}' contains an invalid nox.build: {error}",
            source_name(source)
        ))
    })?;
    crate::core::graph::validate(&project).map_err(|error| {
        Error::Config(format!(
            "source '{}' is not a valid Nox project: {error}",
            source_name(source)
        ))
    })?;
    if let Some(package) = package {
        validate_package_riders(package)?;
    }
    Ok(())
}

fn source_name(source: &PackageSource) -> String {
    match &source.kind {
        PackageSourceKind::GitHub => format!(
            "github:{}/{}",
            source.user.as_deref().unwrap_or("unknown"),
            source.repo.as_deref().unwrap_or("unknown")
        ),
        PackageSourceKind::Registry => {
            format!("pkgs:{}", source.package.as_deref().unwrap_or("unknown"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{add_missing_rider_guidance, missing_rider_requirement};
    use crate::core::error::Error;

    #[test]
    fn explains_missing_riders_for_remote_installs() {
        let error = add_missing_rider_guidance(
            Error::Config("toolchain for D Rider was not found".to_string()),
            "pkgs:ripnet",
        );
        let Error::Config(message) = error else {
            panic!("expected configuration error");
        };
        assert!(message.contains("toolchain for D Rider was not found"));
        assert!(message.contains("ldc2, dmd, or gdc"));
        assert!(message.contains("Remote package and GitHub installation is a new feature"));
        assert!(message.contains("make it available on PATH"));
    }

    #[test]
    fn names_an_explicit_package_rider_command() {
        assert_eq!(
            missing_rider_requirement(
                "Package \"ripnet\" requires Rider \"dmd\", but it was not found."
            ),
            Some("This package explicitly requires the 'dmd' command on PATH.".to_string())
        );
    }

    #[test]
    fn leaves_unrelated_install_errors_unchanged() {
        let error = Error::Config("invalid nox.build".to_string());
        let result = add_missing_rider_guidance(error, "github:user/repo");
        let Error::Config(message) = result else {
            panic!("expected configuration error");
        };
        assert_eq!(message, "invalid nox.build");
    }
}
