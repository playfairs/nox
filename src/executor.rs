use crate::error::{Error, Result};
use crate::model::{Project, Target, TargetKind};
use crate::state::BuildState;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;

pub fn build(project: &Project, state: &BuildState, jobs: usize) -> Result<()> {
  let order = crate::graph::order(project)?;
  let jobs = jobs.max(1);
  for name in order {
    let target = project
      .target(&name)
      .ok_or_else(|| Error::Config(format!("missing target '{name}'")))?;
    build_target(project, target, state, jobs)?;
  }
  Ok(())
}

fn build_target(project: &Project, target: &Target, state: &BuildState, jobs: usize) -> Result<()> {
  let output_dir = state
    .build_dir
    .join(&state.configuration)
    .join(&target.name);
  fs::create_dir_all(&output_dir)?;
  if matches!(
    target.kind,
    TargetKind::RustExecutable | TargetKind::RustLibrary
  ) {
    let rustc = crate::toolchain::detect_rust()?;
    let source = target
      .sources
      .first()
      .ok_or_else(|| Error::Config("Rust target has no source".to_string()))?;
    let output = artifact_path(&output_dir, target);
    let mut command = Command::new(rustc);
    command
      .arg(source)
      .arg("-o")
      .arg(&output)
      .args(&target.flags);
    if target.kind == TargetKind::RustLibrary {
      command.args(["--crate-type", "lib"]);
    }
    run(command)?;
    println!("built {}", output.display());
    return Ok(());
  }
  let sources = Arc::new(target.sources.clone());
  let next = Arc::new(std::sync::Mutex::new(0usize));
  let mut threads = Vec::new();
  for _ in 0..jobs.min(sources.len().max(1)) {
    let sources = Arc::clone(&sources);
    let next = Arc::clone(&next);
    let target = target.clone();
    let state = state.clone();
    let output_dir = output_dir.clone();
    threads.push(thread::spawn(move || -> Result<Vec<PathBuf>> {
      let mut outputs = Vec::new();
      loop {
        let index = {
          let mut value = next
            .lock()
            .map_err(|_| Error::Process("worker lock poisoned".to_string()))?;
          if *value >= sources.len() {
            None
          } else {
            let result = *value;
            *value += 1;
            Some(result)
          }
        };
        let Some(index) = index else { break };
        outputs.push(compile(&sources[index], &target, &state, &output_dir)?);
      }
      Ok(outputs)
    }));
  }
  let mut objects = Vec::new();
  for thread in threads {
    objects.extend(
      thread
        .join()
        .map_err(|_| Error::Process("build worker panicked".to_string()))??,
    );
  }
  let output = artifact_path(&output_dir, target);
  let mut args = target.linker_flags.clone();
  match target.kind {
    TargetKind::StaticLibrary => {
      let mut command = Command::new(&state.archiver);
      command.arg("-rcs").arg(&output).args(&objects);
      run(command)?;
    }
    TargetKind::SharedLibrary => {
      args.insert(0, "-shared".to_string());
      let linker = linker_for_target(target, &state.linker);
      let mut command = Command::new(linker);
      command.args(args).args(&objects).arg("-o").arg(&output);
      run(command)?;
    }
    TargetKind::Executable => {
      let linker = linker_for_target(target, &state.linker);
      let mut command = Command::new(linker);
      command.args(args).args(&objects);
      for dependency in &target.dependencies {
        command.arg(
          project
            .target(dependency)
            .map(|item| {
              artifact_path(
                &state.build_dir.join(&state.configuration).join(&item.name),
                item,
              )
            })
            .ok_or_else(|| Error::Config(format!("unknown dependency '{dependency}'")))?,
        );
      }
      command.arg("-o").arg(&output);
      run(command)?;
    }
    TargetKind::RustExecutable | TargetKind::RustLibrary => unreachable!(),
  }
  println!("built {}", output.display());
  Ok(())
}

fn compile(
  source: &Path,
  target: &Target,
  state: &BuildState,
  output_dir: &Path,
) -> Result<PathBuf> {
  let stem = source
    .file_stem()
    .and_then(|value| value.to_str())
    .ok_or_else(|| Error::Config(format!("invalid source '{}'", source.display())))?;
  let object = output_dir.join(format!("{stem}.o"));
  let depfile = output_dir.join(format!("{stem}.d"));
  if object_needs_build(&object, source, &depfile) {
    let compiler = if is_cpp_source(source) {
      crate::toolchain::detect_cpp()
    } else {
      state.compiler.clone()
    };
    let mut command = Command::new(compiler);
    command
      .arg("-MMD")
      .arg("-MF")
      .arg(&depfile)
      .arg("-c")
      .arg(source)
      .arg("-o")
      .arg(&object);
    if state.configuration == "debug" {
      command.arg("-g").arg("-O0");
    } else {
      command.arg("-O2");
    }
    command
      .args(&target.flags)
      .args(target.defines.iter().map(|define| format!("-D{define}")))
      .args(
        target
          .include_dirs
          .iter()
          .map(|directory| format!("-I{}", directory.display())),
      );
    run(command)?;
    println!("compiled {}", source.display());
  }
  Ok(object)
}

fn artifact_path(directory: &Path, target: &Target) -> PathBuf {
  let prefix = match target.kind {
    TargetKind::StaticLibrary | TargetKind::SharedLibrary => "lib",
    _ => "",
  };
  let suffix = match target.kind {
    TargetKind::StaticLibrary => ".a",
    TargetKind::SharedLibrary => ".so",
    _ => "",
  };
  directory.join(format!("{prefix}{}{suffix}", target.name))
}

fn is_cpp_source(source: &Path) -> bool {
  matches!(
    source.extension().and_then(|value| value.to_str()),
    Some("cpp" | "cc" | "cxx")
  )
}

fn linker_for_target(target: &Target, c_linker: &str) -> String {
  if target.sources.iter().any(|source| is_cpp_source(source)) {
    crate::toolchain::detect_cpp()
  } else {
    c_linker.to_string()
  }
}

fn object_needs_build(object: &Path, source: &Path, depfile: &Path) -> bool {
  let Ok(object_time) = fs::metadata(object).and_then(|metadata| metadata.modified()) else {
    return true;
  };
  let source_newer = fs::metadata(source)
    .and_then(|metadata| metadata.modified())
    .map(|time| time > object_time)
    .unwrap_or(true);
  if source_newer || !depfile.exists() {
    return true;
  }
  fs::read_to_string(depfile)
    .ok()
    .map(|text| {
      text.split_whitespace().skip(1).any(|path| {
        fs::metadata(path.trim_end_matches('\\'))
          .and_then(|metadata| metadata.modified())
          .map(|time| time > object_time)
          .unwrap_or(false)
      })
    })
    .unwrap_or(true)
}

fn run(mut command: Command) -> Result<()> {
  command.stdin(Stdio::null());
  let display = format_command(&command);
  let status = command
    .status()
    .map_err(|error| Error::Process(format!("{display}: {error}")))?;
  if status.success() {
    Ok(())
  } else {
    Err(Error::Process(format!("{display} exited with {status}")))
  }
}

fn format_command(command: &Command) -> String {
  format!("{:?}", command)
}

impl Clone for BuildState {
  fn clone(&self) -> Self {
    Self {
      root: self.root.clone(),
      build_dir: self.build_dir.clone(),
      configuration: self.configuration.clone(),
      compiler: self.compiler.clone(),
      linker: self.linker.clone(),
      archiver: self.archiver.clone(),
    }
  }
}
