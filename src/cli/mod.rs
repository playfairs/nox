use crate::build_system::executor;
use crate::build_system::state::BuildState;
use crate::core::error::{Error, Result};
use crate::core::graph;
use crate::core::model::{Target, TargetKind};
use crate::core::output;
use crate::project::parser;
use crate::project_root;
use crate::run;
use crate::task;
use crate::toolchain::{detection, rider};
use std::fs;
use std::path::{Path, PathBuf};

mod help;

fn version() -> &'static str {
    include_str!("../../VERSION").trim()
}

pub fn run() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let first = arguments.next();
    if matches!(first.as_deref(), Some("--version" | "-V" | "-v")) {
        output::version("nox", version());
        return Ok(());
    }
    let command = match first.as_deref() {
        Some("--help" | "-h") | None => "help".to_string(),
        Some(value) => value.to_string(),
    };
    let mut build_dir = PathBuf::from("build");
    let mut build_dir_explicit = false;
    let mut configuration = "debug".to_string();
    let mut prefix = default_install_prefix();
    let mut help_requested = false;
    let mut reconfigure = false;
    let mut compile_flags = Vec::new();
    let mut jobs = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut positional = Vec::new();
    let mut run_arguments = Vec::new();
    while let Some(argument) = arguments.next() {
        if command == "run" && argument == "--" {
            run_arguments.extend(arguments);
            break;
        }
        match argument.as_str() {
            "--help" | "-h" => help_requested = true,
            "--version" | "-V" | "-v" => {
                output::version("nox", version());
                return Ok(());
            }
            "-j" => {
                jobs = arguments
                    .next()
                    .ok_or_else(|| Error::Config("-j requires a value".to_string()))?
                    .parse()
                    .map_err(|_| Error::Config("invalid job count".to_string()))?
            }
            value if value.starts_with("-j") && value.len() > 2 => {
                jobs = value[2..]
                    .parse()
                    .map_err(|_| Error::Config("invalid job count".to_string()))?
            }
            "--release" => configuration = "release".to_string(),
            "--debug" => configuration = "debug".to_string(),
            "--reconfigure" => reconfigure = true,
            "-C" | "--build-dir" => {
                build_dir_explicit = true;
                build_dir = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--build-dir requires a path".to_string()))?,
                )
            }
            "--compile-flag" => compile_flags.push(
                arguments
                    .next()
                    .ok_or_else(|| Error::Config("--compile-flag requires a value".to_string()))?,
            ),
            "--prefix" => {
                prefix = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--prefix requires a path".to_string()))?,
                )
            }
            value if value.starts_with('-') => {
                return Err(Error::Config(format!("unknown option '{value}'")));
            }
            value => positional.push(value.to_string()),
        }
    }
    if let Some(value) = positional.first() {
        if matches!(command.as_str(), "setup" | "build" | "compile") {
            build_dir_explicit = true;
            build_dir = PathBuf::from(value);
        }
    }
    let root = project_root()?;
    if !build_dir_explicit && !matches!(command.as_str(), "setup" | "configure") {
        if let Some(configured_build_dir) = configured_build_dir(&root)? {
            build_dir = configured_build_dir;
        }
    }
    let state_dir = if build_dir.is_absolute() {
        build_dir
    } else {
        root.join(build_dir)
    };
    if help_requested {
        help::print(&command);
        return Ok(());
    }
    match command.as_str() {
        "setup" | "configure" => {
            if reconfigure && state_dir.exists() {
                fs::remove_dir_all(&state_dir)?;
            }
            setup(&root, &state_dir, &configuration, compile_flags)
        }
        "build" | "compile" => {
            let mut state = BuildState::load(&state_dir)?;
            state.compile_flags.extend(compile_flags);
            let project = parser::parse_file(&state.root.join("nox.build"))?;
            graph::validate(&project)?;
            executor::build(&project, &state, jobs)
        }
        "clean" => {
            if state_dir.exists() {
                fs::remove_dir_all(state_dir)?;
            }
            if !build_dir_explicit {
                remove_project_config(&root)?;
            }
            Ok(())
        }
        "rebuild" => {
            if state_dir.exists() {
                fs::remove_dir_all(&state_dir)?;
            }
            setup(&root, &state_dir, &configuration, compile_flags)?;
            let state = BuildState::load(&state_dir)?;
            let project = parser::parse_file(&root.join("nox.build"))?;
            executor::build(&project, &state, jobs)
        }
        "targets" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            for target in project.targets {
                output::item(&target.name);
            }
            Ok(())
        }
        "list" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            for target in project.targets {
                output::item(&target.name);
            }
            Ok(())
        }
        "validate" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            graph::validate(&project)?;
            output::action("validated", project.name);
            Ok(())
        }
        "status" | "stat" => status(&state_dir),
        "riders" => {
            for rider in rider::available() {
                output::list_item(rider.name, rider.description);
            }
            Ok(())
        }
        "graph" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            for name in graph::order(&project)? {
                output::item(name);
            }
            Ok(())
        }
        "run" => {
            let code = run::execute(
                positional.first().map(String::as_str),
                &run_arguments,
                &root,
                &state_dir,
                &configuration,
                jobs,
            )?;
            if code == 0 {
                Ok(())
            } else {
                Err(Error::Exit(code))
            }
        }
        "test" => task::run_task(&root.join("noxfile"), "test"),
        "install" => {
            let prefix = if prefix.is_absolute() {
                prefix
            } else {
                root.join(prefix)
            };
            install(&root, &state_dir, &configuration, jobs, &prefix)
        }
        "uninstall" => {
            let prefix = if prefix.is_absolute() {
                prefix
            } else {
                root.join(prefix)
            };
            uninstall(&root, &prefix)
        }
        "task" => task::run_task(
            &root.join("noxfile"),
            positional
                .first()
                .ok_or_else(|| Error::Config("task requires a name".to_string()))?,
        ),
        "help" => {
            help::print(positional.first().map(String::as_str).unwrap_or(""));
            Ok(())
        }
        "version" => {
            output::version("nox", version());
            Ok(())
        }
        "bump-version" | "bump" => bump_version(
            &root,
            positional.first().map(String::as_str),
            positional.len(),
            version_files(&root)?,
        ),
        value => Err(Error::Config(format!("unknown command '{value}'"))),
    }
}

fn bump_version(
    root: &Path,
    requested: Option<&str>,
    argument_count: usize,
    specified_files: Option<Vec<PathBuf>>,
) -> Result<()> {
    if argument_count > 1 {
        return Err(Error::Config(
            "bump-version accepts at most one version or release type".to_string(),
        ));
    }

    let version_path = root.join("VERSION");
    if !version_path.is_file() {
        return Err(Error::Config("VERSION file is required".to_string()));
    }
    let current = fs::read_to_string(&version_path)?.trim().to_string();
    let current_parts = parse_version(&current)?;
    let next = match requested.unwrap_or("patch") {
        "major" => format!("{}.0.0", current_parts[0] + 1),
        "minor" => format!("{}.{}.0", current_parts[0], current_parts[1] + 1),
        "patch" => format!(
            "{}.{}.{}",
            current_parts[0],
            current_parts[1],
            current_parts[2] + 1
        ),
        value => {
            parse_version(value)?;
            value.to_string()
        }
    };

    let mut updated_files = Vec::new();
    match specified_files {
        Some(paths) => {
            update_version_file(&version_path, &current, &next, &mut updated_files)?;
            for path in paths {
                update_version_file(&root.join(path), &current, &next, &mut updated_files)?;
            }
        }
        None => update_version_references(root, &current, &next, &mut updated_files)?,
    }
    output::action(
        "bumped version",
        format!("{current} -> {next} ({} files)", updated_files.len()),
    );
    for path in &updated_files {
        let display = path.strip_prefix(root).unwrap_or(path);
        output::path_value("updated", display.display());
    }
    Ok(())
}

fn version_files(root: &Path) -> Result<Option<Vec<PathBuf>>> {
    let path = root.join("nox.build");
    if !path.is_file() {
        return Ok(None);
    }
    Ok(parser::parse_file(&path)?.version_files)
}

fn parse_version(value: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(Error::Config(format!(
            "version '{value}' must use MAJOR.MINOR.PATCH format"
        )));
    }
    let parsed = parts
        .iter()
        .map(|part| part.parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| {
            Error::Config(format!(
                "version '{value}' must use numeric MAJOR.MINOR.PATCH components"
            ))
        })?;
    Ok([parsed[0], parsed[1], parsed[2]])
}

fn update_version_references(
    directory: &Path,
    current: &str,
    next: &str,
    updated_files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(".git" | "build" | "target")
            ) {
                continue;
            }
            update_version_references(&path, current, next, updated_files)?;
            continue;
        }
        update_version_file(&path, current, next, updated_files)?;
    }
    Ok(())
}

fn update_version_file(
    path: &Path,
    current: &str,
    next: &str,
    updated_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(());
    };
    let replaced = replace_version_tokens(&contents, current, next);
    if replaced != contents {
        fs::write(path, replaced)?;
        updated_files.push(path.to_path_buf());
    }
    Ok(())
}

fn replace_version_tokens(contents: &str, current: &str, next: &str) -> String {
    let mut result = String::with_capacity(contents.len());
    let mut remaining = contents;
    while let Some(index) = remaining.find(current) {
        let (before, after_match) = remaining.split_at(index);
        let after = &after_match[current.len()..];
        let before_boundary = before
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_ascii_digit() && character != '.');
        let after_boundary = after
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_digit() && character != '.');
        result.push_str(before);
        if before_boundary && after_boundary {
            result.push_str(next);
        } else {
            result.push_str(current);
        }
        remaining = after;
    }
    result.push_str(remaining);
    result
}

fn setup(
    root: &Path,
    build_dir: &Path,
    configuration: &str,
    compile_flags: Vec<String>,
) -> Result<()> {
    let project = parser::parse_file(&root.join("nox.build"))?;
    graph::validate(&project)?;
    let (compiler, linker, archiver) = detection::detect_c();
    let state = BuildState {
        root: root.to_path_buf(),
        build_dir: build_dir.to_path_buf(),
        configuration: configuration.to_string(),
        compiler,
        linker,
        archiver,
        compile_flags,
    };
    state.save()?;
    write_project_config(root, build_dir)?;
    output::configured(project.name, project.version, build_dir.display());
    Ok(())
}

fn project_config_path(root: &Path) -> PathBuf {
    root.join("nox.config")
}

fn configured_build_dir(root: &Path) -> Result<Option<PathBuf>> {
    let path = project_config_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    let value = text
        .strip_prefix("build_dir=")
        .ok_or_else(|| Error::Config("invalid nox.config".to_string()))?
        .trim();
    if value.is_empty() {
        return Err(Error::Config(
            "nox.config has no build directory".to_string(),
        ));
    }
    let build_dir = PathBuf::from(value);
    Ok(Some(if build_dir.is_absolute() {
        build_dir
    } else {
        root.join(build_dir)
    }))
}

fn write_project_config(root: &Path, build_dir: &Path) -> Result<()> {
    let configured_path = build_dir.strip_prefix(root).unwrap_or(build_dir);
    fs::write(
        project_config_path(root),
        format!("build_dir={}\n", configured_path.display()),
    )?;
    Ok(())
}

fn remove_project_config(root: &Path) -> Result<()> {
    let path = project_config_path(root);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn install(
    root: &Path,
    build_dir: &Path,
    configuration: &str,
    jobs: usize,
    prefix: &Path,
) -> Result<()> {
    let needs_setup = !BuildState::path(build_dir).exists()
        || BuildState::load(build_dir)?.configuration != configuration;
    if needs_setup {
        setup(root, build_dir, configuration, Vec::new())?;
    }
    let state = BuildState::load(build_dir)?;
    let project = parser::parse_file(&root.join("nox.build"))?;
    graph::validate(&project)?;
    executor::build(&project, &state, jobs)?;
    for target in project.targets.iter().filter(|target| target.install) {
        let source = executor::target_artifact_path(
            &state
                .build_dir
                .join(&state.configuration)
                .join(&target.name),
            target,
        );
        let destination_dir = install_directory(prefix, target);
        fs::create_dir_all(&destination_dir)?;
        let destination = destination_dir.join(source.file_name().ok_or_else(|| {
            Error::Config(format!("invalid artifact path for '{}'", target.name))
        })?);
        fs::copy(source, &destination)?;
        output::action("installed", destination.display());
    }
    Ok(())
}

fn uninstall(root: &Path, prefix: &Path) -> Result<()> {
    let project = parser::parse_file(&root.join("nox.build"))?;
    graph::validate(&project)?;
    for target in project.targets.iter().filter(|target| target.install) {
        let artifact = executor::target_artifact_path(&PathBuf::from("."), target);
        let destination =
            install_directory(prefix, target).join(artifact.file_name().ok_or_else(|| {
                Error::Config(format!("invalid artifact path for '{}'", target.name))
            })?);
        if destination.exists() {
            fs::remove_file(&destination)?;
            output::action("uninstalled", destination.display());
        }
    }
    Ok(())
}

fn status(build_dir: &Path) -> Result<()> {
    if !BuildState::path(build_dir).exists() {
        output::warning(format!("not configured: {}", build_dir.display()));
        return Ok(());
    }
    let state = BuildState::load(build_dir)?;
    let project = parser::parse_file(&state.root.join("nox.build"))?;
    output::key_value("project", format!("{} {}", project.name, project.version));
    if !project.description.is_empty() {
        output::key_value("description", &project.description);
    }
    if !project.license.is_empty() {
        output::key_value("license", &project.license);
    }
    output::key_value("edition", &project.edition);
    output::key_value("dependencies", project.dependencies.len());
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

fn install_directory(prefix: &Path, target: &Target) -> PathBuf {
    match target.kind {
        TargetKind::StaticLibrary | TargetKind::SharedLibrary => prefix.join("lib"),
        _ => prefix.join("bin"),
    }
}

fn default_install_prefix() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\Program Files\Nox")
    } else {
        PathBuf::from("/usr/local")
    }
}

#[cfg(test)]
mod tests {
    use super::{bump_version, version_files};
    use std::fs;

    #[test]
    fn bumps_version_references_in_project_files() {
        let root = std::env::temp_dir().join(format!("nox-version-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test directory");
        fs::write(root.join("VERSION"), "9.8.7\n").expect("write VERSION");
        fs::write(root.join("Cargo.toml"), "version = \"9.8.7\"\n").expect("write Cargo.toml");
        fs::write(root.join("README"), "old 9.8.7 and 19.8.7\n").expect("write README");
        fs::write(
            root.join("nox.build"),
            "project \"fixture\" {\n    version_files = [\"Cargo.toml\"]\n    executable \"fixture\" { sources = [\"main.c\"] }\n}\n",
        )
        .expect("write nox.build");

        bump_version(
            &root,
            None,
            0,
            version_files(&root).expect("read version files"),
        )
        .expect("bump version");

        assert_eq!(fs::read_to_string(root.join("VERSION")).unwrap(), "9.8.8\n");
        assert_eq!(
            fs::read_to_string(root.join("Cargo.toml")).unwrap(),
            "version = \"9.8.8\"\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("README")).unwrap(),
            "old 9.8.7 and 19.8.7\n"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
