use crate::core::error::{Error, Result};
use crate::core::model::Setting;
use crate::project::parser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default)]
struct Task {
    dependencies: Vec<String>,
    commands: Vec<String>,
}

struct TaskContext {
    version: String,
    settings: HashMap<String, Setting>,
}

pub fn run_task(path: &Path, name: &str, build_dir: &Path) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let tasks = parse_tasks(&text)?;
    let tasks = if tasks.is_empty() {
        legacy_tasks(&text)
    } else {
        tasks
    };
    let context = if path.with_file_name("nox.build").is_file() {
        let project = parser::parse_file(&path.with_file_name("nox.build"))?;
        TaskContext {
            version: project.version,
            settings: project.settings,
        }
    } else {
        TaskContext {
            version: include_str!("../../VERSION").trim().to_string(),
            settings: HashMap::new(),
        }
    };
    let mut completed = HashSet::new();
    let mut active = Vec::new();
    let build_dir = build_dir.to_string_lossy();
    run_named_task(
        name,
        &tasks,
        &context,
        &build_dir,
        &mut completed,
        &mut active,
    )?;
    crate::core::output::success(format!("task '{name}' completed"));
    Ok(())
}

pub fn list_tasks(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    let tasks = parse_tasks(&text)?;
    let tasks = if tasks.is_empty() {
        legacy_tasks(&text)
    } else {
        tasks
    };
    let mut names: Vec<String> = tasks.into_keys().collect();
    names.sort();
    Ok(names)
}

fn run_named_task(
    name: &str,
    tasks: &HashMap<String, Task>,
    context: &TaskContext,
    build_dir: &str,
    completed: &mut HashSet<String>,
    active: &mut Vec<String>,
) -> Result<()> {
    if completed.contains(name) {
        return Ok(());
    }
    if active.iter().any(|task| task == name) {
        return Err(Error::Config(format!(
            "task dependency cycle: {} -> {name}",
            active.join(" -> ")
        )));
    }
    let task = tasks
        .get(name)
        .ok_or_else(|| Error::Config(format!("task '{name}' was not found")))?;
    active.push(name.to_string());
    for dependency in &task.dependencies {
        if tasks.contains_key(dependency) {
            run_named_task(dependency, tasks, context, build_dir, completed, active)?;
        } else {
            run_nox_command(dependency)?;
        }
    }
    for command in &task.commands {
        run_command(&interpolate(command, context, build_dir), context)?;
    }
    active.pop();
    completed.insert(name.to_string());
    Ok(())
}

fn run_nox_command(command: &str) -> Result<()> {
    let mut parts = command.split_whitespace();
    let Some(name) = parts.next() else {
        return Ok(());
    };
    let arguments: Vec<String> = parts.map(str::to_string).collect();
    let executable = std::env::current_exe()?;
    let mut process = Command::new(executable);
    process.arg(name).args(arguments);
    run_process(&mut process)
}

fn run_command(command: &str, context: &TaskContext) -> Result<()> {
    let mut command_process;
    if cfg!(windows) {
        let shell = match context.settings.get("windows-shell") {
            Some(Setting::List(values)) if values.len() >= 2 => values,
            _ => return run_process(Command::new("cmd").args(["/C", command])),
        };
        command_process = Command::new(&shell[0]);
        command_process.args(&shell[1..]).arg(command);
    } else {
        command_process = Command::new("sh");
        command_process.args(["-c", command]);
    }
    run_process(&mut command_process)
}

fn run_process(command: &mut Command) -> Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Process(format!("task command exited with {status}")))
    }
}

fn interpolate(command: &str, context: &TaskContext, build_dir: &str) -> String {
    let mut result = command.replace("{{build_dir}}", build_dir);
    result = result.replace("{{version}}", &context.version);
    for (name, setting) in &context.settings {
        if let Setting::String(value) = setting {
            result = result.replace(&format!("{{{{{name}}}}}"), value);
        }
    }
    result
}

fn parse_tasks(text: &str) -> Result<HashMap<String, Task>> {
    let mut tasks = HashMap::new();
    let mut in_tasks = false;
    let mut tasks_indent = 0;
    let mut current_name = None;
    let mut task_indent = 0;
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if trimmed == "tasks:" {
            in_tasks = true;
            tasks_indent = indent;
            index += 1;
            continue;
        }
        if !in_tasks {
            index += 1;
            continue;
        }
        if indent <= tasks_indent {
            break;
        }
        if indent > tasks_indent && trimmed.ends_with(':') && !trimmed.starts_with("run:") {
            current_name = Some(trimmed.trim_end_matches(':').to_string());
            task_indent = indent;
            tasks.entry(current_name.clone().unwrap()).or_default();
            index += 1;
            continue;
        }
        let Some(name) = current_name.as_ref() else {
            return Err(Error::Parse(
                "task entry appears before a task name".to_string(),
            ));
        };
        if indent <= task_indent {
            index += 1;
            continue;
        }
        let task: &mut Task = tasks.get_mut(name).expect("task was inserted above");
        if let Some(dependency) = trimmed.strip_prefix("@nox") {
            let dependency = dependency.trim();
            if !dependency.is_empty() {
                task.dependencies.push(dependency.to_string());
            }
        } else if let Some(value) = trimmed.strip_prefix("run:") {
            let run_indent = indent;
            let value = value.trim();
            let mut command = if value == "|" || value == ">" {
                String::new()
            } else {
                unquote(value)
            };
            index += 1;
            while index < lines.len() {
                let continuation = lines[index];
                let continuation_trimmed = continuation.trim();
                let continuation_indent = continuation.len() - continuation.trim_start().len();
                if continuation_trimmed.is_empty() {
                    index += 1;
                    continue;
                }
                if continuation_indent <= run_indent {
                    break;
                }
                if !command.is_empty() {
                    command.push('\n');
                }
                command.push_str(continuation_trimmed);
                index += 1;
            }
            if !command.is_empty() {
                task.commands.push(command);
            }
            continue;
        }
        index += 1;
    }
    Ok(tasks)
}

fn legacy_tasks(text: &str) -> HashMap<String, Task> {
    let mut tasks = HashMap::new();
    for block in text.split("task \"").skip(1) {
        let Some((name, body)) = block.split_once('"') else {
            continue;
        };
        let Some(command) = body
            .split_once("run = \"")
            .and_then(|(_, value)| value.split('"').next())
        else {
            continue;
        };
        tasks.insert(
            name.to_string(),
            Task {
                dependencies: Vec::new(),
                commands: vec![command.to_string()],
            },
        );
    }
    tasks
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{list_tasks, parse_tasks};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_dependencies_repeated_runs_and_multiline_commands() {
        let tasks = parse_tasks(
            "tasks:\n  build:\n    run: cargo build\n  app:\n    @nox build\n    run: first\n    run: |\n      second\n        third\n",
        )
        .expect("tasks should parse");
        assert_eq!(tasks["app"].dependencies, ["build"]);
        assert_eq!(tasks["app"].commands, ["first", "second\nthird"]);
    }

    #[test]
    fn lists_sorted_yaml_tasks() {
        let path = std::env::temp_dir().join(format!(
            "noxfile-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be valid")
                .as_nanos()
        ));
        fs::write(
            &path,
            "tasks:\n  test:\n    run: cargo test\n  build:\n    run: cargo build\n",
        )
        .expect("temporary noxfile should be writable");

        assert_eq!(
            list_tasks(&path).expect("tasks should list"),
            ["build", "test"]
        );
        fs::remove_file(path).expect("temporary noxfile should be removable");
    }
}
