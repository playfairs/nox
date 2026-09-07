use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BuildState {
  pub root: PathBuf,
  pub build_dir: PathBuf,
  pub configuration: String,
  pub compiler: String,
  pub linker: String,
  pub archiver: String,
}

impl BuildState {
  pub fn path(build_dir: &Path) -> PathBuf {
    build_dir.join("nox.state")
  }

  pub fn save(&self) -> Result<()> {
    fs::create_dir_all(&self.build_dir)?;
    let content = format!(
      "root={}\nconfiguration={}\ncompiler={}\nlinker={}\narchiver={}\n",
      self.root.display(),
      self.configuration,
      self.compiler,
      self.linker,
      self.archiver
    );
    fs::write(Self::path(&self.build_dir), content)?;
    Ok(())
  }

  pub fn load(build_dir: &Path) -> Result<Self> {
    let text = fs::read_to_string(Self::path(build_dir)).map_err(|_| {
      Error::Config(format!(
        "'{}' is not configured; run nox setup {}",
        build_dir.display(),
        build_dir.display()
      ))
    })?;
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
      let (key, value) = line
        .split_once('=')
        .ok_or_else(|| Error::Config("invalid build state".to_string()))?;
      values.insert(key, value.to_string());
    }
    Ok(Self {
      root: PathBuf::from(
        values
          .remove("root")
          .ok_or_else(|| Error::Config("build state has no root".to_string()))?,
      ),
      build_dir: build_dir.to_path_buf(),
      configuration: values
        .remove("configuration")
        .unwrap_or_else(|| "debug".to_string()),
      compiler: values
        .remove("compiler")
        .unwrap_or_else(|| "cc".to_string()),
      linker: values.remove("linker").unwrap_or_else(|| "cc".to_string()),
      archiver: values
        .remove("archiver")
        .unwrap_or_else(|| "ar".to_string()),
    })
  }
}
