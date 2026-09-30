use crate::core::error::{Error, Result};
use crate::core::output;
use crate::pkgs::fetch::{fetch_github_source, fetch_registry_index, fetch_source_for_install};
use crate::pkgs::{Package, PackageSource, PackageSourceKind};
use crate::project::parser;
use crate::toolchain::detection;
use std::path::Path;

pub fn run_package_command(arguments: &[String]) -> Result<()> {
    let registry = fetch_registry_index()?;
    match arguments.split_first() {
        None => {
            for package in registry.list() {
                output::item(format!("{}{}", package.name, package.description.as_ref().map(|description| format!(" - {description}")).unwrap_or_default()));
            }
            return Ok(());
        }
        Some((command, rest)) => match command.as_str() {
            "search" => {
                let query = rest.first().map(String::as_str).unwrap_or("");
                let matches = registry.search(query);
                if matches.is_empty() {
                    return Err(Error::Config(format!(
                        "no packages matched '{query}' in the Nox package registry"
                    )));
                }
                for package in matches {
                    output::item(format!("{}{}", package.name, package.description.as_ref().map(|description| format!(" - {description}")).unwrap_or_default()));
                }
                Ok(())
            }
            "info" => {
                let package_name = rest.first().ok_or_else(|| {
                    Error::Config("package info requires a package name".to_string())
                })?;
                let package = registry.resolve(package_name).map_err(|error| Error::Config(error.to_string()))?;
                print_package(package);
                Ok(())
            }
            _ => {
                let package = registry.resolve(command).map_err(|error| Error::Config(error.to_string()))?;
                print_package(package);
                Ok(())
            }
        },
    }
}

fn print_package(package: &Package) {
    println!("{}", package.name);
    if let Some(description) = &package.description {
        println!();
        println!("{description}");
    }
    println!();
    output::key_value("Source", package.url.as_str());
    if let Some(language) = &package.language {
        output::key_value("Language", language);
    }
    let commands = package.riders.commands.join(", ");
    if !commands.is_empty() {
        output::key_value("Riders", commands);
    }
}

pub fn install_from_reference(reference: &str, prefix: &Path, configuration: &str, jobs: usize) -> Result<()> {
    let source = fetch_source_for_install(reference)?;
    install_source(&source, prefix, configuration, jobs, None)
        .map_err(|error| add_missing_rider_guidance(error, reference))
}

fn add_missing_rider_guidance(error: Error, reference: &str) -> Error {
    match error {
        Error::Config(message)
            if message.contains("Rider")
                && (message.contains("not found") || message.contains("was not found")) =>
        {
            let requirement = missing_rider_requirement(&message)
                .unwrap_or_else(|| "Check the package's Rider requirements and project language.".to_string());
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

fn install_source(
    source: &PackageSource,
    prefix: &Path,
    configuration: &str,
    jobs: usize,
    package: Option<&Package>,
) -> Result<()> {
    match source.kind {
        PackageSourceKind::GitHub => {
            let repository = fetch_github_source(source)?;
            let root = repository.root();
            validate_source_root(root, source, package)?;
            crate::cli::install_project(root, &root.join("build"), configuration, jobs, prefix, None)
        }
        PackageSourceKind::Registry => {
            let package_name = source.package.as_deref().ok_or_else(|| {
                Error::Config("package registry sources must include a package name".to_string())
            })?;
            let registry = fetch_registry_index()?;
            let package = registry.resolve(package_name).map_err(|error| Error::Config(error.to_string()))?;
            validate_package_riders(package)?;
            let source = PackageSource::parse(&package.url).map_err(|error| Error::Config(error.to_string()))?;
            install_source(&source, prefix, configuration, jobs, Some(package))
        }
    }
}

fn validate_package_riders(package: &Package) -> Result<()> {
    for requirement in &package.riders.commands {
        let requirement = requirement.trim();
        if requirement.is_empty() {
            continue;
        }
        if detection::require(&[requirement], &format!(
            "Package \"{}\" requires Rider \"{}\"",
            package.name, requirement
        ))
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

fn validate_source_root(root: &Path, source: &PackageSource, package: Option<&Package>) -> Result<()> {
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
        PackageSourceKind::Registry => format!("pkgs:{}", source.package.as_deref().unwrap_or("unknown")),
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
