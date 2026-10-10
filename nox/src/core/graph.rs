use crate::core::error::{Error, Result};
use crate::core::model::Project;
use std::collections::{HashMap, HashSet};

pub fn order(project: &Project) -> Result<Vec<String>> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    for target in &project.targets {
        visit(
            &target.name,
            project,
            &mut visiting,
            &mut visited,
            &mut result,
        )?;
    }
    Ok(result)
}

fn visit(
    name: &str,
    project: &Project,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    result: &mut Vec<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_string()) {
        return Err(Error::Config(format!(
            "dependency cycle involving '{name}'"
        )));
    }
    let target = project
        .target(name)
        .ok_or_else(|| Error::Config(format!("unknown target '{name}'")))?;
    for dependency in &target.dependencies {
        visit(dependency, project, visiting, visited, result)?;
    }
    visiting.remove(name);
    visited.insert(name.to_string());
    result.push(name.to_string());
    Ok(())
}

pub fn validate(project: &Project) -> Result<()> {
    let mut names = HashMap::new();
    for target in &project.targets {
        if names.insert(&target.name, ()).is_some() {
            return Err(Error::Config(format!("duplicate target '{}'", target.name)));
        }
    }
    order(project).map(|_| ())
}
