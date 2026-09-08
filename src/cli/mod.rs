use crate::build_system::executor;
use crate::build_system::state::BuildState;
use crate::core::error::{Error, Result};
use crate::core::graph;
use crate::core::model::{Target, TargetKind};
use crate::core::output;
use crate::project::parser;
use crate::project_root;
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
    let mut configuration = "debug".to_string();
    let mut prefix = default_install_prefix();
    let mut help_requested = false;
    let mut reconfigure = false;
    let mut compile_flags = Vec::new();
    let mut jobs = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut positional = Vec::new();
    while let Some(argument) = arguments.next() {
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
            build_dir = PathBuf::from(value);
        }
    }
    let root = project_root()?;
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
            let state = BuildState::load(&state_dir)?;
            let project = parser::parse_file(&root.join("nox.build"))?;
            executor::build(&project, &state, jobs)?;
            let target = positional
                .first()
                .ok_or_else(|| Error::Config("run requires a target".to_string()))?;
            let target_model = project
                .target(target)
                .ok_or_else(|| Error::Config(format!("unknown target '{target}'")))?;
            let path = executor::target_artifact_path(
                &state.build_dir.join(&state.configuration).join(target),
                target_model,
            );
            let rider = target_model
                .sources
                .first()
                .and_then(|source| rider::for_source(source.as_path()))
                .ok_or_else(|| Error::Config(format!("no Rider recognizes target '{target}'")))?;
            let mut command = match rider.kind {
                rider::RiderKind::Java | rider::RiderKind::Kotlin => {
                    let mut command = std::process::Command::new("java");
                    command.args(["-jar"]).arg(&path);
                    command
                }
                rider::RiderKind::Python => {
                    let mut command =
                        std::process::Command::new(detection::detect_rider(rider.kind)?);
                    command.arg(&path);
                    command
                }
                rider::RiderKind::JavaScript | rider::RiderKind::TypeScript => {
                    let mut command = std::process::Command::new("node");
                    command.arg(&path);
                    command
                }
                _ if cfg!(windows) => {
                    let mut command = std::process::Command::new("cmd");
                    command.args(["/C"]).arg(&path);
                    command
                }
                _ => std::process::Command::new(&path),
            };
            command.args(positional.iter().skip(1));
            command.status()?;
            Ok(())
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
        value => Err(Error::Config(format!("unknown command '{value}'"))),
    }
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
    output::configured(project.name, project.version, build_dir.display());
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
