use serde::Deserialize;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub mod fetch;
pub mod handlers;
pub mod database;

pub const REGISTRY_URL: &str = "https://pkgs.noxbuild.cc/packages.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageSourceKind {
    GitHub,
    Registry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageSource {
    pub kind: PackageSourceKind,
    pub user: Option<String>,
    pub repo: Option<String>,
    pub package: Option<String>,
}

impl PackageSource {
    pub fn parse(value: &str) -> Result<Self, PackageRegistryError> {
        let value = value.trim();
        if let Some(repository) = value.strip_prefix("github:") {
            let repository = repository.trim();
            let Some((user, repo)) = repository.split_once('/') else {
                return Err(PackageRegistryError::InvalidSource(format!(
                    "invalid GitHub source '{value}' (expected github:user/repo)"
                )));
            };
            let user = user.trim();
            let repo = repo.trim();
            if user.is_empty() || repo.is_empty() {
                return Err(PackageRegistryError::InvalidSource(format!(
                    "invalid GitHub source '{value}' (expected github:user/repo)"
                )));
            }
            return Ok(Self {
                kind: PackageSourceKind::GitHub,
                user: Some(user.to_string()),
                repo: Some(repo.to_string()),
                package: None,
            });
        }
        if let Some(package) = value.strip_prefix("pkgs:") {
            let package = package.trim();
            if package.is_empty() {
                return Err(PackageRegistryError::InvalidSource(
                    "invalid package source 'pkgs:' (expected pkgs:package)".to_string(),
                ));
            }
            return Ok(Self {
                kind: PackageSourceKind::Registry,
                user: None,
                repo: None,
                package: Some(package.to_string()),
            });
        }
        Err(PackageRegistryError::InvalidSource(format!(
            "unsupported source '{value}' (expected github:user/repo or pkgs:package)"
        )))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRiders {
    #[serde(default)]
    pub commands: Vec<String>,
}

impl PackageRiders {
    pub fn from_commands(commands: Vec<&str>) -> Self {
        Self {
            commands: commands.into_iter().map(str::to_string).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub language: Option<String>,
    #[serde(default)]
    pub riders: PackageRiders,
}

impl Package {
    pub fn validate(&self) -> Result<(), PackageRegistryError> {
        if self.name.trim().is_empty() {
            return Err(PackageRegistryError::MalformedPackage(
                "package name cannot be empty".to_string(),
            ));
        }
        if self.url.trim().is_empty() {
            return Err(PackageRegistryError::MalformedPackage(format!(
                "package '{}' is missing a url",
                self.name
            )));
        }
        let _ = PackageSource::parse(&self.url)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PackageRegistryData {
    pub packages: Vec<Package>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageRegistry {
    pub packages: Vec<Package>,
}

impl PackageRegistry {
    pub fn from_str(text: &str) -> Result<Self, PackageRegistryError> {
        let root: serde_json::Value = serde_json::from_str(text)
            .map_err(|error| PackageRegistryError::MalformedRegistry(error.to_string()))?;
        let entries = root
            .get("packages")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                PackageRegistryError::MalformedRegistry(
                    "registry must contain a 'packages' array".to_string(),
                )
            })?;
        let mut seen = HashSet::new();
        let mut packages = Vec::new();
        for entry in entries {
            let package: Package = serde_json::from_value(entry.clone()).map_err(|error| {
                PackageRegistryError::MalformedPackage(error.to_string())
            })?;
            package.validate()?;
            if !seen.insert(package.name.clone()) {
                return Err(PackageRegistryError::DuplicatePackage(format!(
                    "duplicate package '{}' in the registry",
                    package.name
                )));
            }
            packages.push(package);
        }
        Ok(Self { packages })
    }

    pub fn resolve(&self, package_name: &str) -> Result<&Package, PackageRegistryError> {
        let requested = package_name.trim();
        self.packages
            .iter()
            .find(|package| package.name.eq_ignore_ascii_case(requested))
            .ok_or_else(|| {
                PackageRegistryError::PackageNotFound(format!(
                    "package \"{requested}\" was not found in the Nox package registry"
                ))
            })
    }

    pub fn search(&self, query: &str) -> Vec<&Package> {
        let query = query.trim();
        if query.is_empty() {
            return self.packages.iter().collect();
        }
        self.packages
            .iter()
            .filter(|package| {
                package.name.to_ascii_lowercase().contains(&query.to_ascii_lowercase())
                    || package
                        .description
                        .as_deref()
                        .is_some_and(|description| description.to_ascii_lowercase().contains(&query.to_ascii_lowercase()))
            })
            .collect()
    }

    pub fn list(&self) -> &[Package] {
        &self.packages
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageRegistryError {
    MalformedRegistry(String),
    MalformedPackage(String),
    DuplicatePackage(String),
    PackageNotFound(String),
    InvalidSource(String),
}

impl Display for PackageRegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedRegistry(message) => write!(formatter, "malformed registry: {message}"),
            Self::MalformedPackage(message) => write!(formatter, "malformed package: {message}"),
            Self::DuplicatePackage(message) => write!(formatter, "duplicate package: {message}"),
            Self::PackageNotFound(message) => write!(formatter, "{message}"),
            Self::InvalidSource(message) => write!(formatter, "invalid package source: {message}"),
        }
    }
}

impl std::error::Error for PackageRegistryError {}
