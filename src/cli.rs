use crate::error::{Error, Result};
use crate::executor;
use crate::graph;
use crate::model::TargetKind;
use crate::parser;
use crate::project_root;
use crate::state::BuildState;
use crate::task;
use crate::toolchain;
use std::fs;
use std::path::{Path, PathBuf};

fn version() -> &'static str {
    include_str!("../VERSION").trim()
}

pub fn run() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let first = arguments.next();
    if matches!(first.as_deref(), Some("--version" | "-V" | "-v")) {
        println!("nox {}", version());
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
    let mut jobs = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut positional = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => help_requested = true,
            "--version" | "-V" | "-v" => {
                println!("nox {}", version());
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
            "--build-dir" => {
                build_dir = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--build-dir requires a path".to_string()))?,
                )
            }
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
        if command == "setup" || command == "build" {
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
        print_command_help(&command);
        return Ok(());
    }
    match command.as_str() {
        "setup" | "configure" => setup(&root, &state_dir, &configuration),
        "build" => {
            let state = BuildState::load(&state_dir)?;
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
            setup(&root, &state_dir, &configuration)?;
            let state = BuildState::load(&state_dir)?;
            let project = parser::parse_file(&root.join("nox.build"))?;
            executor::build(&project, &state, jobs)
        }
        "targets" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            for target in project.targets {
                println!("{}", target.name);
            }
            Ok(())
        }
        "list" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            for target in project.targets {
                println!("{}", target.name);
            }
            Ok(())
        }
        "validate" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            graph::validate(&project)?;
            println!("validated {}", project.name);
            Ok(())
        }
        "status" | "stat" => status(&state_dir),
        "riders" => {
            for rider in crate::rider::available() {
                println!("{}: {}", rider.name, rider.description);
            }
            Ok(())
        }
        "graph" => {
            let project = parser::parse_file(&root.join("nox.build"))?;
            for name in graph::order(&project)? {
                println!("{name}");
            }
            Ok(())
        }
        "run" => {
            let state = BuildState::load(&state_dir)?;
            let project = parser::parse_file(&root.join("nox.build"))?;
            executor::build(&project, &state, jobs)?;
            let target = positional
                .get(1)
                .or_else(|| positional.first())
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
                .and_then(|source| crate::rider::for_source(source.as_path()))
                .ok_or_else(|| Error::Config(format!("no Rider recognizes target '{target}'")))?;
            let mut command = match rider.kind {
                crate::rider::RiderKind::Java | crate::rider::RiderKind::Kotlin => {
                    let mut command = std::process::Command::new("java");
                    command.args(["-jar"]).arg(&path);
                    command
                }
                crate::rider::RiderKind::Python => {
                    let mut command =
                        std::process::Command::new(crate::toolchain::detect_rider(rider.kind)?);
                    command.arg(&path);
                    command
                }
                crate::rider::RiderKind::JavaScript | crate::rider::RiderKind::TypeScript => {
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
            print_command_help(positional.first().map(String::as_str).unwrap_or(""));
            Ok(())
        }
        "version" => {
            println!("nox {}", version());
            Ok(())
        }
        value => Err(Error::Config(format!("unknown command '{value}'"))),
    }
}

fn print_command_help(command: &str) {
    let text = match command {
        "" => {
            "The Nox Build System\n\nUsage: nox <COMMAND> [OPTIONS]\n\nCommands:\n  setup, configure  Configure a build directory\n  build             Build configured targets\n  rebuild           Clean, configure, and build\n  clean             Remove build artifacts\n  install           Build and install targets\n  uninstall         Remove installed targets\n  validate          Validate nox.build\n  status            Show configuration status\n  riders            List language and toolchain Riders\n  targets, list     List targets\n  graph             Show dependency order\n  run               Build and run an executable\n  test              Run the noxfile test task\n  task              Run a noxfile task\n  version           Print the Nox version\n  help              Show command help\n\nOptions:\n  -h, --help       Show help\n  -v, --version    Show version\n\nRun 'nox <COMMAND> --help' for command-specific help."
        }
        "setup" | "configure" => {
            "Usage: nox setup [BUILD_DIR] [--release|--debug]\n\nParse nox.build, validate the graph, detect toolchains, and write build state.\nThe default build directory is build. Alias: configure."
        }
        "build" => {
            "Usage: nox build [BUILD_DIR] [-j N] [--release|--debug]\n\nLoad build state, compile changed sources, and link targets incrementally."
        }
        "rebuild" => {
            "Usage: nox rebuild [--release|--debug]\n\nRemove the build directory, configure it again, and build all targets."
        }
        "clean" => {
            "Usage: nox clean\n\nRemove the configured build directory. Installed files are preserved."
        }
        "install" => {
            "Usage: nox install [--prefix PATH] [--release|--debug]\n\nConfigure if needed, build, and install targets marked install = true.\nExecutables go to <prefix>/bin and libraries to <prefix>/lib."
        }
        "uninstall" => {
            "Usage: nox uninstall [--prefix PATH]\n\nRemove targets marked install = true from the installation prefix."
        }
        "validate" => {
            "Usage: nox validate\n\nParse nox.build and validate target names and dependency cycles without building."
        }
        "status" | "stat" => {
            "Usage: nox status\n\nShow whether the build directory is configured and display its active configuration. Alias: stat."
        }
        "riders" => "Usage: nox riders\n\nList the language and toolchain Riders available to Nox.",
        "targets" | "list" => "Usage: nox targets\n\nList declared targets. Alias: list.",
        "graph" => "Usage: nox graph\n\nPrint targets in dependency order.",
        "run" => "Usage: nox run TARGET\n\nBuild the project and run the named executable target.",
        "test" => "Usage: nox test\n\nRun task \"test\" from noxfile.",
        "task" => "Usage: nox task NAME\n\nRun a named task from noxfile.",
        "version" => "Usage: nox version\n\nPrint the Nox version.",
        "help" => "Usage: nox help [COMMAND]\n\nShow general or command-specific help.",
        _ => "Unknown command. Run 'nox --help' to list available commands.",
    };
    println!("{text}");
}

fn setup(root: &Path, build_dir: &Path, configuration: &str) -> Result<()> {
    let project = parser::parse_file(&root.join("nox.build"))?;
    graph::validate(&project)?;
    let (compiler, linker, archiver) = toolchain::detect_c();
    let state = BuildState {
        root: root.to_path_buf(),
        build_dir: build_dir.to_path_buf(),
        configuration: configuration.to_string(),
        compiler,
        linker,
        archiver,
    };
    state.save()?;
    println!(
        "configured {} {} in {}",
        project.name,
        project.version,
        build_dir.display()
    );
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
        setup(root, build_dir, configuration)?;
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
        println!("installed {}", destination.display());
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
            println!("uninstalled {}", destination.display());
        }
    }
    Ok(())
}

fn status(build_dir: &Path) -> Result<()> {
    if !BuildState::path(build_dir).exists() {
        println!("not configured: {}", build_dir.display());
        return Ok(());
    }
    let state = BuildState::load(build_dir)?;
    let project = parser::parse_file(&state.root.join("nox.build"))?;
    println!("project: {} {}", project.name, project.version);
    if !project.description.is_empty() {
        println!("description: {}", project.description);
    }
    if !project.license.is_empty() {
        println!("license: {}", project.license);
    }
    println!("edition: {}", project.edition);
    println!("dependencies: {}", project.dependencies.len());
    println!("root: {}", state.root.display());
    println!("build directory: {}", state.build_dir.display());
    println!("configuration: {}", state.configuration);
    println!("compiler: {}", state.compiler);
    println!("linker: {}", state.linker);
    println!("archiver: {}", state.archiver);
    println!("targets: {}", project.targets.len());
    Ok(())
}

fn install_directory(prefix: &Path, target: &crate::model::Target) -> PathBuf {
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
