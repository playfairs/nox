use crate::core::error::{Error, Result};
use crate::pkgs::{PackageRegistry, PackageSource, PackageSourceKind, REGISTRY_URL};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn fetch_registry(url: &str) -> Result<PackageRegistry> {
    let output = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", url])
        .output()
        .map_err(|error| Error::Process(format!("could not fetch the package registry: {error}")))?;
    if !output.status.success() {
        return Err(Error::Process(format!(
            "the package registry at '{url}' could not be fetched (curl exited with {})",
            output.status
        )));
    }
    let text = String::from_utf8(output.stdout).map_err(|error| {
        Error::Process(format!("the package registry response was not valid UTF-8: {error}"))
    })?;
    PackageRegistry::from_str(&text).map_err(|error| Error::Config(error.to_string()))
}

pub fn fetch_registry_index() -> Result<PackageRegistry> {
    fetch_registry(REGISTRY_URL)
}

pub fn fetch_github_source(source: &PackageSource) -> Result<TemporarySource> {
    let (user, repo) = match (&source.user, &source.repo) {
        (Some(user), Some(repo)) => (user, repo),
        _ => {
            return Err(Error::Config(format!(
                "GitHub source '{}' is missing a user and repository name",
                source
                    .user
                    .as_deref()
                    .or(source.package.as_deref())
                    .unwrap_or("unknown")
            )));
        }
    };
    let directory = std::env::temp_dir().join("nox").join("packages").join(format!(
        "{user}-{repo}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| {
                Error::Process(format!("could not create a temporary source directory: {error}"))
            })?
            .as_nanos()
    ));
    fs::create_dir_all(&directory)?;
    let repository = format!("https://github.com/{user}/{repo}.git");
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--quiet",
            &repository,
            &directory.to_string_lossy(),
        ])
        .status()
        .map_err(|error| {
            Error::Process(format!("could not start git for '{repository}': {error}"))
        })?;
    if !status.success() {
        let _ = fs::remove_dir_all(&directory);
        return Err(Error::Config(format!(
            "GitHub source 'github:{user}/{repo}' was not found or is not accessible"
        )));
    }
    Ok(TemporarySource { root: directory })
}

pub fn fetch_source_for_install(reference: &str) -> Result<PackageSource> {
    let source = PackageSource::parse(reference).map_err(|error| Error::Config(error.to_string()))?;
    match source.kind {
        PackageSourceKind::Registry => {
            let package_name = source.package.as_deref().unwrap_or_default();
            let registry = fetch_registry_index()?;
            let package = registry.resolve(package_name).map_err(|error| Error::Config(error.to_string()))?;
            PackageSource::parse(&package.url).map_err(|error| Error::Config(error.to_string()))
        }
        PackageSourceKind::GitHub => Ok(source),
    }
}

pub struct TemporarySource {
    pub root: PathBuf,
}

impl TemporarySource {
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for TemporarySource {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
