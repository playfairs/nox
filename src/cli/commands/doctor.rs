use crate::build_system::state::BuildState;
use crate::core::error::{Error, Result};
use crate::core::model::TargetKind;
use crate::core::output;
use crate::project::parser;
use std::path::Path;

pub fn run(root: &Path, build_dir: &Path, requested_project: Option<&str>) -> Result<()> {
    output::section("overview");
    output::key_value("command", "doctor");
    output::path_value("nox.build", root.join("nox.build").display());
    output::path_value("nox.state", super::super::project_config_path(root).display());
    match super::super::configured_build_dir(root)? {
        Some(configured_dir) => output::path_value("configured build dir", configured_dir.display()),
        None => output::warning("no project build directory is configured in nox.state"),
    }

    if !BuildState::path(build_dir).exists() {
        output::warning(format!("not configured: {}", build_dir.display()));
        return Ok(());
    }

    let state = BuildState::load(build_dir)?;
    let projects = parser::parse_file_projects(&state.root.join("nox.build"))?;
    let Some(project) = select_status_project(&projects, requested_project)? else {
        return Ok(());
    };

    let project_label = project.version.as_deref().map_or_else(
        || project.name.clone(),
        |version| format!("{} {version}", project.name),
    );

    output::blank_line();
    output::section("project");
    output::key_value("project", project_label);
    if !project.description.is_empty() {
        output::key_value("description", &project.description);
    }
    if !project.license.is_empty() {
        output::key_value("license", &project.license);
    }
    if let Some(repository) = &project.repository {
        output::key_value("repository", repository);
    }
    if let Some(website) = &project.website {
        output::key_value("website", website);
    }
    if !project.authors.is_empty() {
        output::key_value("authors", output::github_users(&project.authors));
    }
    if !project.maintainers.is_empty() {
        output::key_value("maintainers", project.maintainers.join(", "));
    }
    output::key_value("edition", &project.edition);
    output::key_value("dependencies", project.dependencies.len());

    output::blank_line();
    output::section("build");
    output::path_value("root", state.root.display());
    output::path_value("build directory", state.build_dir.display());
    output::key_value("configuration", &state.configuration);
    output::key_value("compiler", &state.compiler);
    output::key_value("linker", &state.linker);
    output::key_value("archiver", &state.archiver);
    output::key_value("compile flags", format!("{:?}", state.compile_flags));
    output::key_value("targets", project.targets.len());
    Ok(())
}

fn select_status_project<'a>(
    projects: &'a [crate::core::model::Project],
    requested: Option<&str>,
) -> Result<Option<&'a crate::core::model::Project>> {
    if let Some(name) = requested {
        return projects
            .iter()
            .find(|project| project.name == name)
            .map(Some)
            .ok_or_else(|| {
                Error::Config(format!(
                    "unknown project '{name}'; available projects: {}",
                    projects
                        .iter()
                        .map(|project| project.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            });
    }
    if projects.len() == 1 {
        return Ok(Some(&projects[0]));
    }
    println!("{}", project_selection_message(projects, "doctor"));
    Ok(None)
}

fn project_selection_message(projects: &[crate::core::model::Project], command: &str) -> String {
    let executables = projects
        .iter()
        .flat_map(|project| {
            project.targets.iter().filter_map(|target| {
                matches!(target.kind, TargetKind::Executable { .. }).then(|| {
                    format!(
                        "  {}: {} (nox {} {})",
                        project.name, target.name, command, project.name
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    format!(
        "nox.build defines {} projects and {} executables; specify a project with `nox {} <project>`:\n{}",
        projects.len(),
        executables.len(),
        command,
        executables.join("\n")
    )
}
