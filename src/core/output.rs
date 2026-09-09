use std::fmt::Display;
use std::io::{self, IsTerminal};

const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const MAGENTA: &str = "\x1b[35m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const IRIS: &str = "\x1b[38;2;196;167;231m"; // Comes from https://rosepinetheme.com/palette/iris
const RESET: &str = "\x1b[0m";

fn colorize(color: &str, message: impl Display) -> String {
    let message = message.to_string();
    if io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        format!("{color}{message}{RESET}")
    } else {
        message
    }
}

fn hyperlink(url: &str, text: impl Display) -> String {
    let text = text.to_string();
    if io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
        format!("\x1b]8;;{url}\x1b\\{BOLD}{IRIS}{text}{RESET}\x1b]8;;\x1b\\")
    } else {
        text
    }
}

fn line(message: String) {
    println!("{message}");
}

pub fn action(action: impl Display, subject: impl Display) {
    line(format!(
        "{} {}",
        colorize(&format!("{BOLD}{GREEN}"), action),
        colorize(CYAN, subject)
    ));
}

pub fn configured(project: impl Display, version: impl Display, path: impl Display) {
    line(format!(
        "{} {} {} {} {}",
        colorize(&format!("{BOLD}{GREEN}"), "configured"),
        colorize(CYAN, project),
        colorize(YELLOW, version),
        colorize(BLUE, "in"),
        colorize(CYAN, path)
    ));
}

pub fn item(name: impl Display) {
    line(colorize(YELLOW, name));
}

pub fn list_item(name: impl Display, description: impl Display) {
    line(format!(
        "{}: {}",
        colorize(MAGENTA, name),
        colorize(BLUE, description)
    ));
}

pub fn key_value(key: impl Display, value: impl Display) {
    line(format!(
        "{} {}",
        colorize(&format!("{BOLD}{BLUE}"), format!("{key}:")),
        colorize(GREEN, value)
    ));
}

pub fn path_value(key: impl Display, value: impl Display) {
    line(format!(
        "{} {}",
        colorize(&format!("{BOLD}{BLUE}"), format!("{key}:")),
        colorize(CYAN, value)
    ));
}

pub fn version(name: impl Display, value: impl Display) {
    line(format!(
        "{} {}",
        colorize(CYAN, name),
        colorize(YELLOW, value)
    ));
}

pub fn help(message: &str) {
    for value in message.lines() {
        if value.is_empty() {
            line(String::new());
        } else if value == "The Nox Build System" {
            line(hyperlink("https://github.com/playfairs/nox", value));
        } else if value == "Commands:" || value == "Options:" {
            line(colorize(&format!("{BOLD}{CYAN}"), value));
        } else if let Some(rest) = value.strip_prefix("Usage: ") {
            line(format!(
                "{} {}",
                colorize(&format!("{BOLD}{YELLOW}"), "Usage:"),
                rest
            ));
        } else {
            line(value.to_string());
        }
    }
}

pub fn success(message: impl Display) {
    line(colorize(GREEN, message));
}

pub fn warning(message: impl Display) {
    line(colorize(YELLOW, message));
}

pub fn error(message: impl Display) {
    eprintln!(
        "{}",
        if io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
            format!("{RED}{message}{RESET}")
        } else {
            message.to_string()
        }
    );
}
