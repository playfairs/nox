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
        TargetKind::RustExecutable | TargetKind::RustLibrary | TargetKind::DExecutable
    ) {
        let compiler = if target.kind == TargetKind::DExecutable {
            crate::toolchain::detect_rider(crate::rider::RiderKind::D)?
        } else {
            crate::toolchain::detect_rust()?
        };
        let source = target
            .sources
            .first()
            .ok_or_else(|| Error::Config("Rust target has no source".to_string()))?;
        let output = artifact_path(&output_dir, target);
        let mut command = Command::new(compiler);
        if target.kind == TargetKind::DExecutable {
            command.arg("-J").arg(&state.root);
        }
        if target.kind == TargetKind::DExecutable {
            command.args(&target.sources);
        } else {
            command.arg(source);
        }
        if target.kind == TargetKind::DExecutable {
            command.arg("-of").arg(&output);
        } else {
            command.arg("-o").arg(&output);
        }
        command.args(&target.flags);
        if target.kind == TargetKind::RustLibrary {
            command.args(["--crate-type", "lib"]);
        }
        if target.kind == TargetKind::DExecutable {
            command.args(target.linker_flags.iter().map(|flag| {
                flag.strip_prefix("-l")
                    .map(|library| format!("-L-l{library}"))
                    .unwrap_or_else(|| flag.clone())
            }));
        }
        run(command)?;
        println!("built {}", output.display());
        return Ok(());
    }
    if let Some(rider) = target
        .sources
        .first()
        .and_then(|source| crate::rider::for_source(source))
    {
        if !matches!(
            rider.kind,
            crate::rider::RiderKind::C | crate::rider::RiderKind::Cpp
        ) {
            return build_external_target(target, &output_dir, rider.kind);
        }
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
                        .ok_or_else(|| {
                            Error::Config(format!("unknown dependency '{dependency}'"))
                        })?,
                );
            }
            command.arg("-o").arg(&output);
            run(command)?;
        }
        TargetKind::RustExecutable | TargetKind::RustLibrary | TargetKind::DExecutable => {
            unreachable!()
        }
    }
    println!("built {}", output.display());
    Ok(())
}

fn build_external_target(
    target: &Target,
    output_dir: &Path,
    kind: crate::rider::RiderKind,
) -> Result<()> {
    let tool = crate::toolchain::detect_rider(kind)?;
    let output = external_artifact_path(output_dir, target, kind);
    fs::create_dir_all(output_dir)?;
    match kind {
        crate::rider::RiderKind::Go => {
            let mut command = Command::new(tool);
            command
                .arg("build")
                .arg("-o")
                .arg(&output)
                .args(&target.sources);
            run(command)?;
        }
        crate::rider::RiderKind::D => {
            let mut command = Command::new(&tool);
            if tool.ends_with("gdc") {
                command.args(&target.sources).arg("-o").arg(&output);
            } else {
                command
                    .args(&target.sources)
                    .arg(format!("-of={}", output.display()));
            }
            command.args(&target.flags);
            run(command)?;
        }
        crate::rider::RiderKind::Java => {
            let classes = output_dir.join("classes");
            fs::create_dir_all(&classes)?;
            let mut compile = Command::new(tool);
            compile.arg("-d").arg(&classes).args(&target.sources);
            run(compile)?;
            let mut archive = Command::new("jar");
            archive
                .args(["--create", "--file"])
                .arg(&output)
                .arg("-C")
                .arg(&classes)
                .arg(".");
            run(archive)?;
        }
        crate::rider::RiderKind::CSharp => {
            let mut command = Command::new(tool);
            command
                .arg("-nologo")
                .arg(format!("-out:{}", output.display()))
                .args(&target.sources);
            run(command)?;
        }
        crate::rider::RiderKind::Swift => {
            let mut command = Command::new(tool);
            command.args(&target.sources).arg("-o").arg(&output);
            run(command)?;
        }
        crate::rider::RiderKind::Zig => {
            let source = target
                .sources
                .first()
                .ok_or_else(|| Error::Config("Zig target has no source".to_string()))?;
            let mut command = Command::new(tool);
            command
                .arg("build-exe")
                .arg(source)
                .arg(format!("-femit-bin={}", output.display()));
            run(command)?;
        }
        crate::rider::RiderKind::Python => {
            let source = target
                .sources
                .first()
                .ok_or_else(|| Error::Config("Python target has no source".to_string()))?;
            let mut check = Command::new(tool);
            check.args(["-m", "py_compile"]).arg(source);
            run(check)?;
            fs::copy(source, &output)?;
        }
        crate::rider::RiderKind::JavaScript => {
            let source = target
                .sources
                .first()
                .ok_or_else(|| Error::Config("JavaScript target has no source".to_string()))?;
            let mut check = Command::new(tool);
            check.args(["--check"]).arg(source);
            run(check)?;
            fs::copy(source, &output)?;
        }
        crate::rider::RiderKind::TypeScript => {
            let source = target
                .sources
                .first()
                .ok_or_else(|| Error::Config("TypeScript target has no source".to_string()))?;
            let generated = output_dir.join("generated");
            fs::create_dir_all(&generated)?;
            let mut command = Command::new(tool);
            command
                .args([
                    source.to_string_lossy().as_ref(),
                    "--target",
                    "ES2020",
                    "--module",
                    "commonjs",
                    "--outDir",
                ])
                .arg(&generated);
            run(command)?;
            let javascript = generated
                .join(
                    source
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("main"),
                )
                .with_extension("js");
            fs::copy(javascript, &output)?;
        }
        crate::rider::RiderKind::Kotlin => {
            let mut command = Command::new(tool);
            command
                .args(&target.sources)
                .args(["-include-runtime", "-d"])
                .arg(&output);
            run(command)?;
        }
        crate::rider::RiderKind::C
        | crate::rider::RiderKind::Cpp
        | crate::rider::RiderKind::Rust => {
            unreachable!()
        }
    }
    println!("built {}", output.display());
    Ok(())
}

fn external_artifact_path(
    directory: &Path,
    target: &Target,
    kind: crate::rider::RiderKind,
) -> PathBuf {
    let suffix = match kind {
        crate::rider::RiderKind::Java | crate::rider::RiderKind::Kotlin => ".jar",
        crate::rider::RiderKind::Python => ".py",
        crate::rider::RiderKind::JavaScript | crate::rider::RiderKind::TypeScript => ".js",
        _ => "",
    };
    directory.join(format!("{}{suffix}", target.name))
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
        if crate::rider::for_source(source).is_none() {
            return Err(Error::Config(format!(
                "no Rider recognizes source '{}'",
                source.display()
            )));
        }
        let compiler = if crate::rider::for_source(source)
            .is_some_and(|rider| rider.kind == crate::rider::RiderKind::Cpp)
        {
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

pub fn target_artifact_path(directory: &Path, target: &Target) -> PathBuf {
    target
        .sources
        .first()
        .and_then(|source| crate::rider::for_source(source.as_path()))
        .map(|rider| external_artifact_path(directory, target, rider.kind))
        .unwrap_or_else(|| artifact_path(directory, target))
}

fn linker_for_target(target: &Target, c_linker: &str) -> String {
    if target.sources.iter().any(|source| {
        crate::rider::for_source(source)
            .is_some_and(|rider| rider.kind == crate::rider::RiderKind::Cpp)
    }) {
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
