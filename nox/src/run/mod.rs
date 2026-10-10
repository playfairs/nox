use crate::build_system::executor;
use crate::build_system::state::BuildState;
use crate::core::error::{Error, Result};
use crate::core::graph;
use crate::core::model::{Project, Target, TargetKind};
use crate::project::parser;
use crate::toolchain::{detection, rider};
use noml::{Value, parse as parse_noml};
use std::collections::BTreeMap;
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
    if path.is_dir() && (input == "." || !project_root.join("nox.build").is_file()) {
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
        let projects = parser::parse_file_projects(&state.root.join("nox.build"))?;
        let Some(project) = select_project(&projects, requested_target)? else {
            return Ok(0);
        };
        let requested_target = (projects.len() == 1
            && requested_target.is_some_and(|name| name != project.name))
        .then_some(requested_target)
        .flatten();
        graph::validate(&project)?;
        executor::build(&project, &state, self.jobs)?;
        let target = select_target(&project, requested_target)?;
        if matches!(
            &target.kind,
            TargetKind::Executable {
                language: Some(crate::core::model::TargetLanguage::Gradle)
            }
        ) {
            let mut gradle_arguments = if target.gradle_run_tasks.is_empty() {
                vec![format!(":{}:run", target.name)]
            } else {
                target.gradle_run_tasks.clone()
            };
            gradle_arguments.extend(target.gradle_run_options.clone());
            crate::build_system::gradle::run(&state.root, &gradle_arguments)?;
            return Ok(0);
        }
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

fn select_project<'a>(projects: &'a [Project], requested: Option<&str>) -> Result<Option<&'a Project>> {
    if let Some(name) = requested {
        if let Some(project) = projects.iter().find(|project| project.name == name) {
            return Ok(Some(project));
        }
        if projects.len() == 1 {
            return Ok(Some(&projects[0]));
        }
        return Err(Error::Config(format!(
            "unknown project '{name}'; available projects: {}. Use `nox run <project>`",
            projects
                .iter()
                .map(|project| project.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if projects.len() == 1 {
        return Ok(Some(&projects[0]));
    }
    let executables = projects
        .iter()
        .flat_map(|project| {
            project.targets.iter().filter_map(|target| {
                is_executable(target.kind.clone()).then(|| {
                    format!(
                        "  {}: {} (nox run {})",
                        project.name, target.name, project.name
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    println!(
        "nox.build defines {} projects and {} executables; specify a project with `nox run <project>`:\n{}",
        projects.len(),
        executables.len(),
        executables.join("\n")
    );
    Ok(None)
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

    let runnable: Vec<&Target> = project
        .targets
        .iter()
        .filter(|target| is_executable(target.kind.clone()))
        .collect();

    match runnable.len() {
        0 => Err(Error::Config(
            "project has no runnable executable target".to_string(),
        )),
        1 => Ok(runnable[0]),
        _ => Err(Error::Config(format!(
            "there are {} executables in this project; use `nox targets` to list them and `nox run <name>` to run one",
            runnable.len()
        ))),
    }
}

fn is_executable(kind: TargetKind) -> bool {
    matches!(kind, TargetKind::Executable { .. })
}

#[cfg(test)]
mod tests {
    use super::select_target;
    use crate::core::model::{Project, Target, TargetKind};
    use std::collections::HashMap;

    fn executable(name: &str) -> Target {
        Target {
            name: name.to_string(),
            kind: TargetKind::Executable { language: None },
            sources: vec![],
            dependencies: vec![],
            include_dirs: vec![],
            defines: vec![],
            flags: vec![],
            linker_flags: vec![],
            gradle_tasks: vec![],
            gradle_options: vec![],
            gradle_run_tasks: vec![],
            gradle_run_options: vec![],
            install: false,
        }
    }

    #[test]
    fn selects_single_executable_without_explicit_name() {
        let project = Project {
            name: "demo".to_string(),
            version: None,
            version_files: None,
            description: String::new(),
            license: String::new(),
            edition: "1".to_string(),
            dependencies: vec![],
            repository: None,
            website: None,
            authors: vec![],
            maintainers: vec![],
            targets: vec![executable("app")],
            settings: HashMap::new(),
            extra_env: HashMap::new(),
        };

        assert_eq!(select_target(&project, None).unwrap().name, "app");
    }

    #[test]
    fn rejects_ambiguous_run_without_target_name() {
        let project = Project {
            name: "demo".to_string(),
            version: None,
            version_files: None,
            description: String::new(),
            license: String::new(),
            edition: "1".to_string(),
            dependencies: vec![],
            repository: None,
            website: None,
            authors: vec![],
            maintainers: vec![],
            targets: vec![executable("app"), executable("worker")],
            settings: HashMap::new(),
            extra_env: HashMap::new(),
        };

        let error = select_target(&project, None).unwrap_err().to_string();
        assert!(error.contains("there are 2 executables in this project"));
        assert!(error.contains("nox targets"));
        assert!(error.contains("nox run <name>"));
    }
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
        Some(rider::RiderKind::QSharp) => {
            let dotnet = required_runtime(artifact, &["dotnet".to_string()], "the .NET SDK")?;
            let mut command = Command::new(dotnet);
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

pub fn handler_language(source: &Path) -> Result<String> {
    Ok(HandlerRegistry::resolve(source)?.language)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandlerMode {
    Runtime,
    Compile,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HandlerDefinition {
    language: String,
    extensions: Vec<String>,
    mode: HandlerMode,
    tools: Vec<String>,
    arguments: Vec<String>,
    dependency: String,
}

#[derive(Debug)]
struct HandlerFile {
    handlers: Vec<HandlerDefinition>,
}

impl HandlerFile {
    fn parse_registry() -> Result<Self> {
        let document = parse_noml(include_str!("handlers.noml"))
            .map_err(|error| Error::Config(format!("invalid run handler registry: {error}")))?;
        let ruleset = match document {
            Value::Ruleset(ruleset) => ruleset,
            other => {
                return Err(Error::Config(format!(
                    "invalid run handler registry: expected ruleset root, got {other:?}"
                )))
            }
        };
        let handlers = ruleset
            .entries
            .into_iter()
            .filter(|entry| entry.type_name == "handler")
            .map(|entry| HandlerDefinition::from_noml_entry(entry))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { handlers })
    }
}

impl HandlerDefinition {
    fn from_noml_entry(entry: noml::Entry) -> Result<Self> {
        let fields = entry.properties;
        Ok(Self {
            language: string_field(&fields, "language")?,
            extensions: string_list_field(&fields, "extensions")?,
            mode: mode_field(&fields, "mode")?,
            tools: string_list_field(&fields, "tools")?,
            arguments: string_list_field(&fields, "arguments")?,
            dependency: string_field(&fields, "dependency")?,
        })
    }
}

fn string_field(fields: &BTreeMap<String, Value>, key: &str) -> Result<String> {
    match fields.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(other) => Err(Error::Config(format!(
            "invalid run handler field '{key}': expected string, got {other:?}"
        ))),
        None => Err(Error::Config(format!(
            "invalid run handler field '{key}': missing value"
        ))),
    }
}

fn string_list_field(fields: &BTreeMap<String, Value>, key: &str) -> Result<Vec<String>> {
    match fields.get(key) {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(part) => Ok(part.clone()),
                other => Err(Error::Config(format!(
                    "invalid run handler field '{key}': expected string list, got {other:?}"
                ))),
            })
            .collect(),
        Some(other) => Err(Error::Config(format!(
            "invalid run handler field '{key}': expected array, got {other:?}"
        ))),
        None => Err(Error::Config(format!(
            "invalid run handler field '{key}': missing value"
        ))),
    }
}

fn mode_field(fields: &BTreeMap<String, Value>, key: &str) -> Result<HandlerMode> {
    match fields.get(key) {
        Some(Value::String(value)) => match value.as_str() {
            "runtime" => Ok(HandlerMode::Runtime),
            "compile" => Ok(HandlerMode::Compile),
            "unsupported" => Ok(HandlerMode::Unsupported),
            other => Err(Error::Config(format!(
                "invalid run handler field '{key}': unsupported mode '{other}'"
            ))),
        },
        Some(other) => Err(Error::Config(format!(
            "invalid run handler field '{key}': expected string, got {other:?}"
        ))),
        None => Err(Error::Config(format!(
            "invalid run handler field '{key}': missing value"
        ))),
    }
}

struct HandlerRegistry;

impl HandlerRegistry {
    fn resolve(source: &Path) -> Result<HandlerDefinition> {
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| Error::Config(format!("unknown file type: '{}'", source.display())))?;
        let registry = HandlerFile::parse_registry()?;
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

pub struct TemporaryArtifact {
    directory: PathBuf,
    executable: PathBuf,
}

impl TemporaryArtifact {
    pub fn new(source: &Path) -> Result<Self> {
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

    pub fn path(&self) -> &Path {
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
