use crate::core::error::Result;
use crate::core::output;
use std::path::Path;

pub fn run(root: &Path, build_dir: &Path, requested_project: Option<&str>) -> Result<()> {
    output::warning("'nox stat' is deprecated; use 'nox doctor' instead");
    super::doctor::run(root, build_dir, requested_project)
}
