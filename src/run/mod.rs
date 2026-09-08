use crate::build_system::executor;
use crate::build_system::state::BuildState;
use crate::core::error::{Error, Result};
use crate::core::graph;
use crate::core::model::{Project, Target, TargetKind};
use crate::project::parser;
use crate::toolchain::{detection, rider};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn execute(
    input: Option<&str>,
    arguments: &[String],
    project_root: &Path,
    build_dir: &Path,
    configuration: &str,
    jobs: usize,
) -> Result<i32> {
    let input = input.unwrap_or(".");
    let path = project_root.join(input);
    if path.is_dir() {
        return ProjectRunner::new(build_dir, configuration, jobs).run(None, arguments);
    }
    if path.is_file() {
        return FileRunner::run(&path, arguments);
    }
    if input != "."
        && (input.contains(std::path::MAIN_SEPARATOR) || Path::new(input).extension().is_some())
    {
        return Err(Error::Config(format!(
            "run path '{}' does not exist",
            path.display()
        )));
    }
    ProjectRunner::new(build_dir, configuration, jobs).run(Some(input), arguments)
}

struct ProjectRunner<'a> {
    build_dir: &'a Path,
    configuration: &'a str,
    jobs: usize,
}

impl<'a> ProjectRunner<'a> {
    fn new(build_dir: &'a Path, configuration: &'a str, jobs: usize) -> Self {
        Self {
            build_dir,
            configuration,
            jobs,
        }
    }

    fn run(&self, requested_target: Option<&str>, arguments: &[String]) -> Result<i32> {
        let state = BuildState::load(self.build_dir)?;
        if state.configuration != self.configuration {
            return Err(Error::Config(format!(
                "build directory '{}' is configured for {}, not {}",
                self.build_dir.display(),
                state.configuration,
                self.configuration
            )));
        }
        let project = parser::parse_file(&state.root.join("nox.build"))?;
        graph::validate(&project)?;
        executor::build(&project, &state, self.jobs)?;
        let target = select_target(&project, requested_target)?;
        let artifact = executor::target_artifact_path(
            &state
                .build_dir
                .join(&state.configuration)
                .join(&target.name),
            target,
        );
        run_project_artifact(target, &artifact, arguments)
    }
}

fn select_target<'a>(project: &'a Project, requested: Option<&str>) -> Result<&'a Target> {
    if let Some(name) = requested {
        let target = project
            .target(name)
            .ok_or_else(|| Error::Config(format!("unknown runnable target '{name}'")))?;
        if !is_executable(target.kind.clone()) {
            return Err(Error::Config(format!(
                "target '{name}' is not runnable because it is a library"
            )));
        }
        return Ok(target);
    }
    project
        .targets
        .iter()
        .find(|target| is_executable(target.kind.clone()))
        .ok_or_else(|| Error::Config("project has no runnable executable target".to_string()))
}

fn is_executable(kind: TargetKind) -> bool {
    matches!(
        kind,
        TargetKind::Executable
            | TargetKind::CppExecutable
            | TargetKind::RustExecutable
            | TargetKind::DExecutable
    )
}

fn run_project_artifact(target: &Target, artifact: &Path, arguments: &[String]) -> Result<i32> {
    let mut command = match target
        .sources
        .first()
        .and_then(|source| rider::for_source(source))
        .map(|rider| rider.kind)
    {
        Some(rider::RiderKind::Java | rider::RiderKind::Kotlin) => {
            let java = required_runtime(
                artifact,
                &["java".to_string()],
                "the Java runtime (required to run this project target)",
            )?;
            let mut command = Command::new(java);
            command.args(["-jar"]).arg(artifact);
            command
        }
        Some(rider::RiderKind::Python) => {
            let python = required_runtime(
                artifact,
                &["python3".to_string(), "python".to_string()],
                "Python",
            )?;
            let mut command = Command::new(python);
            command.arg(artifact);
            command
        }
        Some(rider::RiderKind::JavaScript | rider::RiderKind::TypeScript) => {
            let node = required_runtime(artifact, &["node".to_string()], "Node.js")?;
            let mut command = Command::new(node);
            command.arg(artifact);
            command
        }
        _ if cfg!(windows) => {
            let mut command = Command::new("cmd");
            command.args(["/C"]).arg(artifact);
            command
        }
        _ => Command::new(artifact),
    };
    command.args(arguments);
    child_status(command)
}

struct FileRunner;

impl FileRunner {
    fn run(source: &Path, arguments: &[String]) -> Result<i32> {
        let handler = HandlerRegistry::resolve(source)?;
        handler.run(source, arguments)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum HandlerMode {
    Runtime,
    Compile,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize)]
struct HandlerDefinition {
    language: String,
    extensions: Vec<String>,
    mode: HandlerMode,
    tools: Vec<String>,
    arguments: Vec<String>,
    dependency: String,
}

#[derive(Debug, Deserialize)]
struct HandlerFile {
    handlers: Vec<HandlerDefinition>,
}

struct HandlerRegistry;

impl HandlerRegistry {
    fn resolve(source: &Path) -> Result<HandlerDefinition> {
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| Error::Config(format!("unknown file type: '{}'", source.display())))?;
        let registry: HandlerFile = serde_yaml::from_str(include_str!("handlers.yaml"))
            .map_err(|error| Error::Config(format!("invalid run handler registry: {error}")))?;
        registry
            .handlers
            .into_iter()
            .find(|handler| handler.extensions.iter().any(|item| item == extension))
            .ok_or_else(|| {
                Error::Config(format!(
                    "unknown file type for '{}'; no run handler is registered for '.{extension}'",
                    source.display()
                ))
            })
    }
}

impl HandlerDefinition {
    fn run(&self, source: &Path, arguments: &[String]) -> Result<i32> {
        let tool = required_runtime(source, &self.tools, &self.dependency)?;
        match self.mode {
            HandlerMode::Runtime => {
                let mut command = Command::new(tool);
                command
                    .args(self.arguments.iter().map(String::as_str))
                    .arg(source)
                    .args(arguments);
                child_status(command)
            }
            HandlerMode::Compile => {
                let handler = CompileAndRunHandler::new(tool, &self.arguments);
                handler.run(source, arguments)
            }
            HandlerMode::Unsupported => Err(Error::Config(format!(
                "cannot run '{}': the {} handler is registered, but its execution mode is not implemented yet; required dependency {}",
                source.display(),
                self.language,
                self.dependency
            ))),
        }
    }
}

struct CompileAndRunHandler {
    compiler: String,
    arguments: Vec<String>,
}

impl CompileAndRunHandler {
    fn new(compiler: String, arguments: &[String]) -> Self {
        Self {
            compiler,
            arguments: arguments.to_vec(),
        }
    }

    fn run(&self, source: &Path, child_arguments: &[String]) -> Result<i32> {
        let artifact = TemporaryArtifact::new(source)?;
        let mut compile = Command::new(&self.compiler);
        let arguments = self
            .arguments
            .iter()
            .map(|argument| {
                argument
                    .replace("{source}", &source.to_string_lossy())
                    .replace("{output}", &artifact.path().to_string_lossy())
            })
            .collect::<Vec<_>>();
        compile.args(&arguments);
        let status = compile.status().map_err(|error| {
            Error::Process(format!("could not invoke '{}': {error}", self.compiler))
        })?;
        if !status.success() {
            return Err(Error::Process(format!(
                "compiling '{}' with '{}' failed with {status}",
                source.display(),
                self.compiler
            )));
        }
        let mut command = Command::new(artifact.path());
        command.args(child_arguments);
        child_status(command)
    }
}

struct TemporaryArtifact {
    directory: PathBuf,
    executable: PathBuf,
}

impl TemporaryArtifact {
    fn new(source: &Path) -> Result<Self> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                Error::Process(format!("could not create temporary run path: {error}"))
            })?
            .as_nanos();
        let directory = std::env::temp_dir()
            .join("nox")
            .join("run")
            .join(format!("{}-{stamp}", std::process::id()));
        fs::create_dir_all(&directory)?;
        let name = source
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("program");
        Ok(Self {
            executable: directory.join(if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.to_string()
            }),
            directory,
        })
    }

    fn path(&self) -> &Path {
        &self.executable
    }
}

impl Drop for TemporaryArtifact {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn child_status(mut command: Command) -> Result<i32> {
    let display = format!("{command:?}");
    let status = command
        .status()
        .map_err(|error| Error::Process(format!("could not execute {display}: {error}")))?;
    Ok(exit_code(status))
}

fn required_runtime(source: &Path, programs: &[String], dependency: &str) -> Result<String> {
    let candidate_names = programs.join(", ");
    let candidates = programs.iter().map(String::as_str).collect::<Vec<_>>();
    detection::require(&candidates, dependency).map_err(|_| {
        Error::Config(format!(
            "cannot run '{}': required dependency '{candidate_names}' ({dependency}) is not installed or not available on PATH",
            source.display(),
        ))
    })
}

fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::{HandlerRegistry, TemporaryArtifact};
    use std::path::Path;

    #[test]
    fn resolves_supported_file_handlers() {
        assert!(matches!(
            HandlerRegistry::resolve(Path::new("Test.fsx")),
            Ok(handler) if handler.language == "F#"
        ));
        assert!(matches!(
            HandlerRegistry::resolve(Path::new("Test.c")),
            Ok(handler) if handler.language == "C"
        ));
        assert!(matches!(
            HandlerRegistry::resolve(Path::new("Test.cpp")),
            Ok(handler) if handler.language == "C++"
        ));
        assert!(matches!(
            HandlerRegistry::resolve(Path::new("Test.d")),
            Ok(handler) if handler.language == "D"
        ));
        assert!(matches!(
            HandlerRegistry::resolve(Path::new("Test.rb")),
            Ok(handler) if handler.language == "Ruby"
        ));
    }

    #[test]
    fn rejects_unknown_file_handlers() {
        assert!(HandlerRegistry::resolve(Path::new("Test.unknown")).is_err());
    }

    #[test]
    fn temporary_artifacts_live_outside_the_source_tree_and_clean_up() {
        let artifact =
            TemporaryArtifact::new(Path::new("examples/c/Test.c")).expect("temp artifact");
        let directory = artifact.directory.clone();
        assert!(!artifact.path().starts_with(Path::new("examples")));
        assert!(directory.exists());
        drop(artifact);
        assert!(!directory.exists());
    }
}
