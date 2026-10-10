use crate::core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPackageDatabase {
    #[serde(default)]
    pub packages: Vec<InstalledPackage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPackage {
    pub name: String,
    pub description: String,
    pub license: String,
    pub language: Vec<String>,
    pub riders: Vec<String>,
    pub required_commands: Vec<String>,
    pub method: InstallMethod,
    pub url: Option<String>,
    pub path: Option<PathBuf>,
    pub installed_to: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallMethod {
    Local,
    GitHub,
    Registry,
}

impl InstallMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::GitHub => "github",
            Self::Registry => "registry",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallOrigin {
    pub method: InstallMethod,
    pub url: Option<String>,
    pub path: Option<PathBuf>,
}

impl InstalledPackageDatabase {
    pub fn load() -> Result<Self> {
        Self::load_from(&database_path()?)
    }

    pub fn save(&self) -> Result<()> {
        Self::save_to(self, &database_path()?)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(|error| {
            Error::Config(format!(
                "installed package database '{}' is invalid: {error}",
                path.display()
            ))
        })
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let temporary_path = path.with_extension(format!("tmp-{}", std::process::id()));
        let contents = serde_json::to_vec_pretty(self).map_err(|error| {
            Error::Config(format!(
                "could not encode installed package database: {error}"
            ))
        })?;
        fs::write(&temporary_path, contents)?;
        fs::rename(&temporary_path, path)?;
        Ok(())
    }

    pub fn record(&mut self, package: InstalledPackage) {
        let same_installation = |existing: &InstalledPackage| {
            existing.name.eq_ignore_ascii_case(&package.name)
                && existing.method == package.method
                && existing.url == package.url
                && existing.path == package.path
        };
        if let Some(existing) = self
            .packages
            .iter_mut()
            .find(|existing| same_installation(*existing))
        {
            existing.description = package.description;
            existing.license = package.license;
            existing.language = package.language;
            existing.riders = package.riders;
            for destination in package.installed_to {
                if !existing.installed_to.contains(&destination) {
                    existing.installed_to.push(destination);
                }
            }
        } else {
            self.packages.push(package);
        }
    }

    pub fn find(&self, name: &str) -> Vec<&InstalledPackage> {
        self.packages
            .iter()
            .filter(|package| package.name.eq_ignore_ascii_case(name))
            .collect()
    }

    pub fn remove(&mut self, name: &str) -> Vec<InstalledPackage> {
        let mut removed = Vec::new();
        self.packages.retain(|package| {
            if package.name.eq_ignore_ascii_case(name) {
                removed.push(package.clone());
                false
            } else {
                true
            }
        });
        removed
    }
}

pub fn database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("NOX_PACKAGE_DB") {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "macos")]
    let directory = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Application Support/nox"));

    #[cfg(target_os = "windows")]
    let directory = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|app_data| app_data.join("nox"));

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let directory = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/share"))
        })
        .map(|data_home| data_home.join("nox"));

    directory
        .map(|directory| directory.join("installed-packages.json"))
        .ok_or_else(|| Error::Config("could not determine the Nox data directory".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{InstallMethod, InstalledPackage, InstalledPackageDatabase};
    use std::path::{Path, PathBuf};

    fn package(destination: &str) -> InstalledPackage {
        InstalledPackage {
            name: "demo".to_string(),
            description: "Example package".to_string(),
            license: "MIT".to_string(),
            language: vec!["Rust".to_string()],
            riders: vec!["Rust Rider".to_string()],
            required_commands: Vec::new(),
            method: InstallMethod::Local,
            url: None,
            path: Some(PathBuf::from("/tmp/demo")),
            installed_to: vec![PathBuf::from(destination)],
        }
    }

    #[test]
    fn saves_and_loads_install_records() {
        let path = std::env::temp_dir().join(format!(
            "nox-installed-packages-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut database = InstalledPackageDatabase::default();
        database.record(package("/tmp/demo/bin/demo"));
        database.save_to(&path).unwrap();

        let loaded = InstalledPackageDatabase::load_from(&path).unwrap();
        assert_eq!(loaded.packages, database.packages);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn merges_destinations_for_the_same_installation() {
        let mut database = InstalledPackageDatabase::default();
        database.record(package("/tmp/demo/bin/demo"));
        database.record(package("/opt/demo/bin/demo"));

        assert_eq!(database.packages.len(), 1);
        assert_eq!(database.find("DEMO").len(), 1);
        assert_eq!(database.packages[0].installed_to.len(), 2);
    }

    #[test]
    fn removes_all_installation_records_for_a_name() {
        let mut database = InstalledPackageDatabase::default();
        database.record(package("/tmp/demo/bin/demo"));
        let removed = database.remove("Demo");

        assert_eq!(removed.len(), 1);
        assert!(database.find("demo").is_empty());
    }

    #[test]
    fn missing_database_loads_as_empty() {
        let path = Path::new("/tmp/nox-package-db-that-does-not-exist.json");
        assert!(InstalledPackageDatabase::load_from(path)
            .unwrap()
            .packages
            .is_empty());
    }
}
