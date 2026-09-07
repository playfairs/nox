use crate::error::{Error, Result};
use crate::executor;
use crate::graph;
use crate::parser;
use crate::project_root;
use crate::state::BuildState;
use crate::task;
use crate::toolchain;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run() -> Result<()> {
  let mut arguments = std::env::args().skip(1);
  let command = arguments.next().unwrap_or_else(|| "build".to_string());
  let mut build_dir = PathBuf::from("build");
  let mut configuration = "debug".to_string();
  let mut jobs = std::thread::available_parallelism()
    .map(|value| value.get())
    .unwrap_or(1);
  let mut positional = Vec::new();
  while let Some(argument) = arguments.next() {
    match argument.as_str() {
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
  match command.as_str() {
    "setup" => setup(&root, &state_dir, &configuration),
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
      let path = state
        .build_dir
        .join(&state.configuration)
        .join(target)
        .join(target);
      if cfg!(windows) {
        std::process::Command::new("cmd")
          .arg("/C")
          .arg(path)
          .status()?;
      } else {
        std::process::Command::new(path).status()?;
      }
      Ok(())
    }
    "test" => task::run_task(&root.join("noxfile"), "test"),
    "install" => install(&root, &state_dir),
    "task" => task::run_task(
      &root.join("noxfile"),
      positional
        .first()
        .ok_or_else(|| Error::Config("task requires a name".to_string()))?,
    ),
    "help" => {
      println!(
        "The Nox Build System\n\nnox setup [build] [--release]\nnox build [build] [-j N]\nnox clean | rebuild | install | test | run | graph | targets\nnox task NAME"
      );
      Ok(())
    }
    value => Err(Error::Config(format!("unknown command '{value}'"))),
  }
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

fn install(root: &Path, build_dir: &Path) -> Result<()> {
  let state = BuildState::load(build_dir)?;
  let project = parser::parse_file(&root.join("nox.build"))?;
  let prefix = root.join("install").join(&state.configuration);
  for target in project.targets.iter().filter(|target| target.install) {
    let source = state
      .build_dir
      .join(&state.configuration)
      .join(&target.name)
      .join(match target.kind {
        crate::model::TargetKind::StaticLibrary => format!("lib{}.a", target.name),
        crate::model::TargetKind::SharedLibrary => format!("lib{}.so", target.name),
        _ => target.name.clone(),
      });
    fs::create_dir_all(&prefix)?;
    fs::copy(source, prefix.join(target.name.clone()))?;
  }
  Ok(())
}
