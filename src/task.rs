use crate::error::{Error, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_task(path: &Path, name: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let command = yaml_task_command(&text, name)
        .or_else(|| legacy_task_command(&text, name))
        .ok_or_else(|| {
            Error::Config(format!("task '{name}' was not found or has no run command"))
        })?;
    let status = if cfg!(windows) {
        Command::new("cmd").args(["/C", &command]).status()?
    } else {
        Command::new("sh").args(["-c", &command]).status()?
    };
    if status.success() {
        Ok(())
    } else {
        Err(Error::Process(format!(
            "task '{name}' exited with {status}"
        )))
    }
}

fn yaml_task_command(text: &str, name: &str) -> Option<String> {
    let mut in_tasks = false;
    let mut tasks_indentation = 0;
    let mut task_indentation = None;
    let mut current_task = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "tasks:" {
            in_tasks = true;
            tasks_indentation = line.len() - line.trim_start().len();
            continue;
        }
        if !in_tasks {
            continue;
        }
        let indentation = line.len() - line.trim_start().len();
        if indentation > tasks_indentation && trimmed.ends_with(':') && !trimmed.starts_with("run:")
        {
            task_indentation = Some(indentation);
            current_task = Some(trimmed.trim_end_matches(':'));
            continue;
        }
        if task_indentation.is_some_and(|value| indentation > value)
            && current_task == Some(name)
            && trimmed.starts_with("run:")
        {
            let value = trimmed[4..].trim();
            return Some(unquote(value));
        }
    }
    None
}

fn legacy_task_command(text: &str, name: &str) -> Option<String> {
    let marker = format!("task \"{name}\"");
    let start = text.find(&marker)?;
    let block = &text[start..];
    block
        .split("run = \"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .map(str::to_string)
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
