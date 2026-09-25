use crate::build_system::executor;
use crate::build_system::state::BuildState;
use crate::core::error::{Error, Result};
use crate::core::graph;
use crate::core::model::{Target, TargetKind};
use crate::core::output;
use crate::init;
use crate::project::parser;
use crate::project_root;
use crate::rules::base::BaseRules;
use crate::run;
use crate::task;
use crate::toolchain::{detection, rider};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod commands;
mod help;

const REPOSITORY_URL: &str = "https://github.com/playfairs/nox.git";

fn version() -> &'static str {
    include_str!("../../VERSION").trim()
}

fn version_display() -> String {
    format!("{} (channel: {})", version(), install_channel())
}

fn install_channel() -> &'static str {
    option_env!("NOX_INSTALL_CHANNEL").unwrap_or("unknown")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UpdateChannel {
    Stable,
    Dev,
}

impl UpdateChannel {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "stable" => Ok(Self::Stable),
            "dev" => Ok(Self::Dev),
            _ => Err(Error::Config(format!(
                "unknown update channel '{value}' (expected stable or dev)"
            ))),
        }
    }

    fn branch(self) -> &'static str {
        match self {
            Self::Stable => "master",
            Self::Dev => "dev",
        }
    }
}

pub fn run() -> Result<()> {
    let base_rules = BaseRules::load().map_err(Error::Config)?;
    let mut arguments = std::env::args().skip(1);
    let first = arguments.next();
    if matches!(first.as_deref(), Some("--version" | "-V" | "-v")) {
        output::version("nox", version_display());
        return Ok(());
    }
    let command = match first.as_deref() {
        Some("--help" | "-h") | None => "help".to_string(),
        Some(value) => base_rules.command_name(value).unwrap_or(value).to_string(),
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
    let mut init_options = init::Options::default();
    let mut update_channel = None;
    let mut update_version = None;
    let mut update_dev = false;
    while let Some(argument) = arguments.next() {
        if command == "run" && argument == "--" {
            run_arguments.extend(arguments);
            break;
        }
        match argument.as_str() {
            "--help" | "-h" => help_requested = true,
            "--version" if command == "update" => {
                update_version = Some(arguments.next().ok_or_else(|| {
                    Error::Config("--version requires a value for update".to_string())
                })?);
            }
            "--version" | "-V" | "-v" => {
                output::version("nox", version_display());
                return Ok(());
            }
            "--dev" if command == "update" => update_dev = true,
            "--channel" if command == "update" => {
                update_channel = Some(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--channel requires a value".to_string()))?,
                );
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
            "--debug" => {
                configuration = "debug".to_string();
                if matches!(command.as_str(), "build" | "compile" | "run") {
                    unsafe {
                        std::env::set_var("RUST_BACKTRACE", "full");
                    }
                }
            }
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
            "--name" => {
                init_options.name = Some(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--name requires a value".to_string()))?,
                )
            }
            "--language" => {
                init_options.language = Some(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--language requires a value".to_string()))?,
                )
            }
            "--type" => {
                init_options.project_type = Some(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--type requires a value".to_string()))?,
                )
            }
            "--template" => {
                init_options.template = Some(
                    arguments
                        .next()
                        .ok_or_else(|| Error::Config("--template requires a value".to_string()))?,
                )
            }
            "--no-noxfile" => init_options.noxfile = false,
            "--no-nix" => init_options.nix = false,
            "--formatter" => init_options.formatter = true,
            value if value.starts_with('-') => {
                return Err(Error::Config(format!("unknown option '{value}'")));
            }
            value => positional.push(value.to_string()),
        }
    }
    if let Some(value) = positional.first() {
        if command == "init" {
            init_options.project_name = Some(value.clone());
        }
        if matches!(command.as_str(), "setup" | "build" | "compile") {
            build_dir_explicit = true;
            build_dir = PathBuf::from(value);
        }
    }
    if command == "init" {
        if help_requested {
            help::print(&command);
            return Ok(());
        }
        let root = init::resolve_root(init_options.project_name.as_deref())
            .map_err(crate::init::error::Error::into_core)?;
        return init::run(&root, init_options).map_err(crate::init::error::Error::into_core);
    }
    if command == "help" {
        help::print(positional.first().map(String::as_str).unwrap_or(""));
        return Ok(());
    }
    if help_requested {
        help::print(&command);
        return Ok(());
    }
    let command_rule = base_rules
        .command(&command)
        .ok_or_else(|| Error::Config(format!("unknown command '{command}'")))?;
    if base_rules.accepts_files_as_input(&command)
        && positional
            .first()
            .is_some_and(|input| standalone_file(input).is_some())
    {
        let root = std::env::current_dir()?;
        let code = run::execute(
            positional.first().map(String::as_str),
            &run_arguments,
            &root,
            Path::new("build"),
            &configuration,
            jobs,
        )?;
        return if code == 0 {
            Ok(())
        } else {
            Err(Error::Exit(code))
        };
    }
    if command == "version" {
        output::version("nox", version_display());
        return Ok(());
    }
    if command == "update" {
        if update_dev && update_channel.is_some() {
            return Err(Error::Config(
                "--dev and --channel cannot be used together".to_string(),
            ));
        }
        if update_version.is_some() && (update_dev || update_channel.is_some()) {
            return Err(Error::Config(
                "--version cannot be combined with --channel or --dev".to_string(),
            ));
        }
        let channel = if update_dev {
            Some(UpdateChannel::Dev)
        } else {
            update_channel
                .as_deref()
                .map(UpdateChannel::parse)
                .transpose()?
        };
        return update(channel, update_version.as_deref());
    }
    if command == "riders" {
        let mut riders = rider::available().to_vec();
        riders.sort_by(|left, right| left.name.cmp(right.name));
        for rider in riders {
            output::list_item(rider.name, rider.description);
        }
        return Ok(());
    }
    if command == "nomlfmt" {
        let input = positional.first().ok_or_else(|| {
            Error::Config("nomlfmt requires a NOML file or directory".to_string())
        })?;
        if positional.len() > 1 {
            return Err(Error::Config(
                "nomlfmt accepts exactly one file or directory".to_string(),
            ));
        }
        let path = PathBuf::from(input);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };
        return format_noml(&path);
    }
    if command_rule.requires_project {
        let root = project_root()?;
        return run_with_project(
            &command,
            root,
            build_dir,
            build_dir_explicit,
            configuration,
            prefix,
            reconfigure,
            compile_flags,
            jobs,
            positional,
            run_arguments,
            base_rules.requires_initialization(&command),
        );
    }
    Err(Error::Config(format!("unknown command '{command}'")))
}

fn standalone_file(input: &str) -> Option<PathBuf> {
    let path = PathBuf::from(input);
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    path.is_file().then_some(path)
}

fn format_noml(path: &Path) -> Result<()> {
    let files = if path.is_dir() {
        let mut files = Vec::new();
        collect_noml_files(path, &mut files)?;
        files.sort();
        files
    } else if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        return Err(Error::Config(format!(
            "NOML path '{}' does not exist",
            path.display()
        )));
    };

    if files.is_empty() {
        return Err(Error::Config(format!(
            "no .noml files found under '{}'",
            path.display()
        )));
    }

    for file in &files {
        noml::format::format_file_in_place(file).map_err(Error::Config)?;
        output::action("formatted", file.display());
    }
    Ok(())
}

fn collect_noml_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| matches!(name.to_str(), Some(".git" | "target" | "build")))
            {
                continue;
            }
            collect_noml_files(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "noml")
        {
            files.push(path);
        }
    }
    Ok(())
}

fn run_with_project(
    command: &str,
    root: std::path::PathBuf,
    mut build_dir: PathBuf,
    build_dir_explicit: bool,
    configuration: String,
    prefix: PathBuf,
    reconfigure: bool,
    compile_flags: Vec<String>,
    jobs: usize,
    positional: Vec<String>,
    run_arguments: Vec<String>,
    requires_initialization: bool,
) -> Result<()> {
    let project = parser::parse_file(&root.join("nox.build"))?;
    apply_project_environment(&project);
    if !build_dir_explicit && !matches!(command, "setup" | "configure") {
        if let Some(configured_build_dir) = configured_build_dir(&root)? {
            build_dir = configured_build_dir;
        }
    }
    let state_dir = if build_dir.is_absolute() {
        build_dir
    } else {
        root.join(build_dir)
    };
    if requires_initialization && !BuildState::path(&state_dir).is_file() {
        if project_config_path(&root).is_file() {
            setup(&root, &state_dir, &configuration, compile_flags.clone())?;
        } else {
            return Err(Error::Config(format!(
                "'{}' is not configured; run nox setup {}",
                state_dir.display(),
                state_dir.display()
            )));
        }
    }
    match command {
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
        "doctor" | "doc" => {
            commands::doctor::run(&root, &state_dir, positional.first().map(String::as_str))
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
        "env" => env(&root),
        "status" | "stat" => {
            output::warning("'nox stat' is being deprecated soon; use 'nox doctor' instead");
            doctor(&root, &state_dir, positional.first().map(String::as_str))
        }
        "riders" => unreachable!(),
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
        "test" => task::run_task(&root.join("noxfile"), "test", &state_dir),
        "install" => {
            let prefix = if prefix.is_absolute() {
                prefix
            } else {
                root.join(prefix)
            };
            install(
                &root,
                &state_dir,
                &configuration,
                jobs,
                &prefix,
                positional.first().map(String::as_str),
            )
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
            &state_dir,
        ),
        "tasks" => {
            for name in task::list_tasks(&root.join("noxfile"))? {
                output::item(name);
            }
            Ok(())
        }
        "help" => {
            help::print(positional.first().map(String::as_str).unwrap_or(""));
            Ok(())
        }
        "version" => unreachable!(),
        "bump-version" | "bump" => bump_version(
            &root,
            positional.first().map(String::as_str),
            positional.len(),
            version_files(&root)?,
        ),
        value => Err(Error::Config(format!("unknown command '{value}'"))),
    }
}

fn apply_project_environment(project: &crate::core::model::Project) {
    for (name, value) in &project.extra_env {
        unsafe {
            std::env::set_var(name, value);
        }
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

fn update(channel: Option<UpdateChannel>, requested_version: Option<&str>) -> Result<()> {
    if is_nix_managed()? {
        println!(
            "Nox is managed by Nix.\nUpdate Nox through your Nix configuration:\n\n    nix flake update nox\n\nRun this command from your Nix configuration directory."
        );
        return Ok(());
    }

    let requested = requested_version.map(parse_semver).transpose()?;
    let (target_version, install_args, channel) = match (channel, requested) {
        (Some(channel), None) => {
            let latest = fetch_version(channel.branch())?;
            (
                latest,
                vec!["--branch".to_string(), channel.branch().to_string()],
                Some(channel),
            )
        }
        (None, Some(version)) => (
            version.clone(),
            vec!["--tag".to_string(), format!("v{version}")],
            None,
        ),
        (None, None) => {
            let channel = UpdateChannel::Stable;
            let latest = fetch_version(channel.branch())?;
            (
                latest,
                vec!["--branch".to_string(), channel.branch().to_string()],
                Some(channel),
            )
        }
        (Some(_), Some(_)) => unreachable!("update channel and version are mutually exclusive"),
    };

    let current = parse_semver(version())?;
    if requested_version.is_some() {
        if target_version == current {
            println!("Nox {target_version} is already installed.");
            return Ok(());
        }
        if target_version < current {
            println!(
                "Requested version ({target_version}) is older than the installed version ({}).\nNox will not downgrade automatically.",
                version()
            );
            return Ok(());
        }
    } else if target_version == current {
        match channel {
            Some(UpdateChannel::Dev) => println!(
                "Nox is already up to date with the development channel ({target_version})."
            ),
            _ => println!("Nox is already up to date ({target_version})."),
        }
        return Ok(());
    } else if target_version < current {
        match channel {
            Some(UpdateChannel::Dev) => println!(
                "Installed Nox version ({}) is newer than the latest development version ({target_version}).\nNox will not downgrade automatically.",
                version()
            ),
            _ => println!(
                "Installed Nox version ({}) is newer than the latest stable version ({target_version}).\nNox will not downgrade automatically.",
                version()
            ),
        }
        return Ok(());
    }

    println!(
        "A newer {} version of Nox is available: {target_version}.\nUpdating...",
        match channel {
            Some(UpdateChannel::Dev) => "development",
            _ => "stable",
        }
    );
    let executable = invoked_executable()?;
    let install_root = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| Error::Config("could not determine Nox installation root".to_string()))?;
    let mut args = vec![
        "install".to_string(),
        "--git".to_string(),
        REPOSITORY_URL.to_string(),
        "nox".to_string(),
        "--root".to_string(),
        install_root.to_string_lossy().into_owned(),
    ];
    args.extend(install_args);
    args.extend(["--locked".to_string(), "--force".to_string()]);
    let status = Command::new("cargo")
        .args(&args)
        .env(
            "NOX_INSTALL_CHANNEL",
            match channel {
                Some(UpdateChannel::Stable) => "stable",
                Some(UpdateChannel::Dev) => "dev",
                None => "unknown",
            },
        )
        .status()
        .map_err(|error| Error::Process(format!("could not start cargo install: {error}")))?;
    if status.success() {
        output::action("updated nox", target_version);
        Ok(())
    } else {
        Err(Error::Process(format!(
            "cargo install exited with {status}"
        )))
    }
}

fn fetch_version(branch: &str) -> Result<SemVersion> {
    let reference = format!("refs/heads/{branch}");
    let commit = Command::new("git")
        .args(["ls-remote", "--exit-code", REPOSITORY_URL, &reference])
        .output()
        .map_err(|error| {
            Error::Process(format!("could not query the latest Nox commit: {error}"))
        })?;
    if !commit.status.success() {
        return Err(Error::Process(format!(
            "could not query the latest Nox commit for channel '{branch}' (git exited with {})",
            commit.status
        )));
    }
    let commit = String::from_utf8(commit.stdout)
        .map_err(|error| Error::Process(format!("latest Nox commit was not valid UTF-8: {error}")))?
        .split_whitespace()
        .next()
        .ok_or_else(|| Error::Process("latest Nox commit response was empty".to_string()))?
        .to_string();
    let cache_buster = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let url = format!(
        "https://raw.githubusercontent.com/playfairs/nox/{commit}/VERSION?cachebust={cache_buster}"
    );
    let response = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", &url])
        .output()
        .map_err(|error| {
            Error::Process(format!("could not query the latest Nox version: {error}"))
        })?;
    if !response.status.success() {
        return Err(Error::Process(format!(
            "could not query the latest Nox version (curl exited with {})",
            response.status
        )));
    }
    let latest = String::from_utf8(response.stdout).map_err(|error| {
        Error::Process(format!("latest Nox version was not valid UTF-8: {error}"))
    })?;
    parse_semver(latest.trim())
}

fn is_nix_managed() -> Result<bool> {
    let executable = invoked_executable()?;
    let explicit_path = std::env::args_os().next().is_some_and(|argument| {
        let path = Path::new(&argument);
        path.is_absolute()
            || path
                .parent()
                .is_some_and(|parent| parent != Path::new(""))
    });
    let path = fs::canonicalize(&executable).unwrap_or(executable);
    if is_nix_managed_path(&path) {
        return Ok(true);
    }
    if explicit_path {
        return Ok(false);
    }

    for candidate in where_nox_paths() {
        let candidate = fs::canonicalize(candidate).unwrap_or_else(|_| PathBuf::from(""));
        if candidate == path {
            return Ok(is_nix_managed_path(&candidate));
        }
    }
    Ok(false)
}

fn invoked_executable() -> Result<PathBuf> {
    let invoked = std::env::args_os().next().map(PathBuf::from);
    let explicit_path = invoked.as_ref().filter(|path| {
        path.is_absolute()
            || path
                .parent()
                .is_some_and(|parent| parent != Path::new(""))
    });
    Ok(explicit_path.cloned().unwrap_or(std::env::current_exe()?))
}

fn is_nix_managed_path(path: &Path) -> bool {
    path.starts_with("/nix/store") || path.to_string_lossy().contains(".nix-profile/bin/nox")
}

fn where_nox_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(result) = Command::new("where").arg("nox").output() {
        paths.extend(
            String::from_utf8_lossy(&result.stdout)
                .lines()
                .map(PathBuf::from),
        );
    }
    if cfg!(target_os = "macos") {
        if let Ok(result) = Command::new("zsh").args(["-lc", "where nox"]).output() {
            paths.extend(
                String::from_utf8_lossy(&result.stdout)
                    .lines()
                    .map(PathBuf::from),
            );
        }
    }
    paths
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SemVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<String>,
}

impl std::fmt::Display for SemVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.prerelease.is_empty() {
            write!(formatter, "-{}", self.prerelease.join("."))?;
        }
        Ok(())
    }
}

impl Ord for SemVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(
                || match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => compare_prerelease(&self.prerelease, &other.prerelease),
                },
            )
    }
}

impl PartialOrd for SemVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_prerelease(left: &[String], right: &[String]) -> std::cmp::Ordering {
    for (left, right) in left.iter().zip(right) {
        let ordering = match (left.parse::<u64>(), right.parse::<u64>()) {
            (Ok(left), Ok(right)) => left.cmp(&right),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => left.cmp(right),
        };
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn parse_semver(value: &str) -> Result<SemVersion> {
    let value = value.strip_prefix('v').unwrap_or(value);
    let value = value.split_once('+').map_or(value, |(value, _)| value);
    let (core, prerelease) = value.split_once('-').map_or((value, ""), |parts| parts);
    let parts: Vec<_> = core.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(Error::Config(format!(
            "version '{value}' is not valid semantic version"
        )));
    }
    let numbers = parts
        .iter()
        .map(|part| part.parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::Config(format!("version '{value}' is not valid semantic version")))?;
    let prerelease = if prerelease.is_empty() {
        Vec::new()
    } else {
        prerelease.split('.').map(str::to_string).collect()
    };
    Ok(SemVersion {
        major: numbers[0],
        minor: numbers[1],
        patch: numbers[2],
        prerelease,
    })
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
    let build_file = root.join("nox.build");
    let project = parser::parse_file(&build_file).map_err(|error| match error {
        Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => Error::Config(format!(
            "setup could not run: required file 'nox.build' was not found"
        )),
        error => error,
    })?;
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
    output::configured(
        project.name,
        project.version.as_deref(),
        build_dir.display(),
    );
    Ok(())
}

fn project_config_path(root: &Path) -> PathBuf {
    root.join("nox.state")
}

fn legacy_project_config_path(root: &Path) -> PathBuf {
    root.join("nox.config")
}

fn configured_build_dir(root: &Path) -> Result<Option<PathBuf>> {
    let state_path = project_config_path(root);
    let legacy_path = legacy_project_config_path(root);
    let path = if state_path.is_file() {
        state_path
    } else if legacy_path.is_file() {
        legacy_path
    } else {
        return Ok(None);
    };

    let text = fs::read_to_string(&path)?;
    let value = text
        .lines()
        .find_map(|line| line.strip_prefix("build_dir="))
        .ok_or_else(|| {
            Error::Config(format!(
                "invalid {}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ))
        })?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::Config(format!(
            "{} has no build directory",
            path.file_name().unwrap_or_default().to_string_lossy()
        )));
    }
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .map(|value| value.replace("\\\"", "\""))
        .unwrap_or_else(|| trimmed.to_string());
    let build_dir = PathBuf::from(unquoted);
    Ok(Some(if build_dir.is_absolute() {
        build_dir
    } else {
        root.join(build_dir)
    }))
}

fn write_project_config(root: &Path, build_dir: &Path) -> Result<()> {
    let configured_path = build_dir.strip_prefix(root).unwrap_or(build_dir);
    let quoted_root = format!("\"{}\"", root.display());
    let quoted_build = format!("\"{}\"", configured_path.display());
    fs::write(
        project_config_path(root),
        format!(
            "// This file is generated by nox.\n// It is not intended to be manually edited.\nroot={}\nbuild_dir={}\n",
            quoted_root, quoted_build
        ),
    )?;
    if legacy_project_config_path(root).exists() {
        fs::remove_file(legacy_project_config_path(root))?;
    }
    Ok(())
}

fn install(
    root: &Path,
    build_dir: &Path,
    configuration: &str,
    jobs: usize,
    prefix: &Path,
    requested_project: Option<&str>,
) -> Result<()> {
    let needs_setup = !BuildState::path(build_dir).exists()
        || BuildState::load(build_dir)?.configuration != configuration;
    if needs_setup {
        setup(root, build_dir, configuration, Vec::new())?;
    }
    let state = BuildState::load(build_dir)?;
    let projects = parser::parse_file_projects(&root.join("nox.build"))?;
    let Some(project) = select_install_project(&projects, requested_project)? else {
        return Ok(());
    };
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

fn doctor(root: &Path, build_dir: &Path, requested_project: Option<&str>) -> Result<()> {
    commands::doctor::run(root, build_dir, requested_project)
}

fn env(root: &Path) -> Result<()> {
    let project = parser::parse_file(&root.join("nox.build"))?;
    let mut entries: Vec<_> = project.extra_env.iter().collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (name, value) in entries {
        println!("{}={}", name, value);
    }
    Ok(())
}

#[allow(dead_code)]
fn status(root: &Path, build_dir: &Path, requested_project: Option<&str>) -> Result<()> {
    commands::status::run(root, build_dir, requested_project)
}

fn select_install_project<'a>(
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
    println!("{}", project_selection_message(projects, "install"));
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
    use super::{
        bump_version, is_nix_managed_path, parse_semver, run_with_project, standalone_file,
        version_files,
    };
    use crate::rules::base::BaseRules;
    use std::fs;
    use std::path::Path;

    #[test]
    fn resolves_short_command_aliases() {
        let rules = BaseRules::load().expect("embedded base rules should load");
        assert_eq!(rules.command_name("b"), Some("build"));
        assert_eq!(rules.command_name("build"), Some("build"));
        assert_eq!(rules.command_name("r"), Some("run"));
        assert_eq!(rules.command_name("run"), Some("run"));
        assert_eq!(rules.command_name("i"), Some("install"));
        assert_eq!(rules.command_name("install"), Some("install"));
        assert_eq!(rules.command_name("doc"), Some("doctor"));
        assert_eq!(rules.command_name("env"), Some("env"));
        assert_eq!(rules.command_name("debug"), None);
    }

    #[test]
    fn clean_keeps_project_state() {
        let root = std::env::temp_dir().join(format!("nox-clean-state-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp project root");
        fs::write(
            root.join("nox.build"),
            "project \"fixture\" {\n    executable \"fixture\" { sources = [\"main.c\"] }\n}\n",
        )
        .expect("write nox.build");
        fs::write(root.join("nox.state"), "build_dir=\"build\"\n").expect("write nox.state");

        run_with_project(
            "clean",
            root.clone(),
            std::path::PathBuf::from("build"),
            false,
            "debug".to_string(),
            std::path::PathBuf::from("/usr/local"),
            false,
            Vec::new(),
            1,
            Vec::new(),
            Vec::new(),
            false,
        )
        .expect("clean should succeed");

        assert!(
            root.join("nox.state").is_file(),
            "nox.state should remain after clean"
        );
        assert!(
            !root.join("build").exists(),
            "build directory should be removed during clean"
        );
        fs::remove_dir_all(root).expect("cleanup test directory");
    }

    #[test]
    fn reads_quoted_build_dir_from_project_state() {
        let root = std::env::temp_dir().join(format!("nox-state-quoted-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp project root");
        let quoted_dir = root.join("custom build dir");
        fs::write(
            root.join("nox.state"),
            format!("build_dir=\"{}\"\n", quoted_dir.display()),
        )
        .expect("write quoted build dir");

        assert_eq!(
            super::configured_build_dir(&root).expect("read configured build dir"),
            Some(quoted_dir)
        );
        fs::remove_dir_all(root).expect("cleanup quoted-state directory");
    }

    #[test]
    fn recognizes_existing_standalone_files() {
        let path =
            std::env::temp_dir().join(format!("nox-standalone-run-{}.py", std::process::id()));
        fs::write(&path, "print('ok')\n").expect("write standalone file");
        assert_eq!(standalone_file(path.to_str().unwrap()), Some(path.clone()));
        fs::remove_file(path).expect("remove standalone file");
    }

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

    #[test]
    fn compares_semantic_versions_and_prereleases() {
        assert!(parse_semver("1.2.10").unwrap() > parse_semver("1.2.9").unwrap());
        assert!(parse_semver("1.2.5").unwrap() > parse_semver("1.2.5-dev").unwrap());
        assert!(parse_semver("1.2.5-10").unwrap() > parse_semver("1.2.5-2").unwrap());
    }

    #[test]
    fn identifies_nix_paths_without_matching_unrelated_installations() {
        assert!(is_nix_managed_path(Path::new(
            "/Users/playfairs/.nix-profile/bin/nox"
        )));
        assert!(is_nix_managed_path(Path::new("/nix/store/nox/bin/nox")));
        assert!(!is_nix_managed_path(Path::new("/usr/local/bin/nox")));
    }
}
