use crate::core::error::{Error, Result};
use crate::core::model::{Project, Target, TargetKind};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
enum Token {
    Word(String),
    String(String),
    Symbol(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    String(String),
    Bool(bool),
    List(Vec<Value>),
    Reference(String),
}

pub fn parse_file(path: &Path) -> Result<Project> {
    let text = fs::read_to_string(path)?;
    parse(&text, path.parent().unwrap_or(Path::new(".")))
}

pub fn parse(text: &str, root: &Path) -> Result<Project> {
    let tokens = lex(text)?;
    let mut parser = Parser {
        tokens,
        position: 0,
        root,
        bindings: HashMap::new(),
    };
    parser.project()
}

fn lex(text: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character.is_whitespace() {
            continue;
        }
        if character == '#' {
            while chars.next().is_some_and(|value| value != '\n') {}
            continue;
        }
        if character == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    while chars.next().is_some_and(|value| value != '\n') {}
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut closed = false;
                    while let Some(value) = chars.next() {
                        if value == '*' && chars.next_if_eq(&'/').is_some() {
                            closed = true;
                            break;
                        }
                    }
                    if !closed {
                        return Err(Error::Parse("unterminated block comment".to_string()));
                    }
                    continue;
                }
                _ => {}
            }
        }
        if "{}[]=(),".contains(character) {
            tokens.push(Token::Symbol(character));
            continue;
        }
        if character == '"' {
            let mut value = String::new();
            let mut closed = false;
            while let Some(next) = chars.next() {
                if next == '"' {
                    closed = true;
                    break;
                }
                if next == '\\' {
                    value.push(
                        chars
                            .next()
                            .ok_or_else(|| Error::Parse("unterminated string".to_string()))?,
                    );
                } else {
                    value.push(next);
                }
            }
            if !closed {
                return Err(Error::Parse("unterminated string".to_string()));
            }
            tokens.push(Token::String(value));
            continue;
        }
        let mut value = String::from(character);
        while let Some(next) = chars.peek().copied() {
            if next.is_whitespace() || "{}[]=(),\"".contains(next) {
                break;
            }
            value.push(next);
            chars.next();
        }
        tokens.push(Token::Word(value));
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    position: usize,
    root: &'a Path,
    bindings: HashMap<String, Value>,
}

impl<'a> Parser<'a> {
    fn project(&mut self) -> Result<Project> {
        self.expect_word("project")?;
        let name = self.string_or_word()?;
        self.expect_symbol('{')?;
        let mut version = include_str!("../../VERSION").trim().to_string();
        let mut description = String::new();
        let mut license = String::new();
        let mut edition = "1".to_string();
        let mut dependencies = Vec::new();
        let mut targets = Vec::new();
        while !self.take_symbol('}') {
            match self.word()?.as_str() {
                "let" => self.binding()?,
                "version" => {
                    self.expect_symbol('=')?;
                    version = self.string_or_file()?;
                }
                "description" => {
                    self.expect_symbol('=')?;
                    description = self.string_or_word()?;
                }
                "license" => {
                    self.expect_symbol('=')?;
                    license = self.string_or_word()?;
                }
                "edition" => {
                    self.expect_symbol('=')?;
                    edition = self.string_or_word()?;
                }
                "dependencies" => {
                    self.expect_symbol('=')?;
                    dependencies = self.strings()?;
                }
                "executable" => targets.push(self.target(TargetKind::Executable)?),
                "cxx_executable" => targets.push(self.target(TargetKind::CppExecutable)?),
                "static_library" | "static" => {
                    targets.push(self.target(TargetKind::StaticLibrary)?)
                }
                "shared_library" | "shared" => {
                    targets.push(self.target(TargetKind::SharedLibrary)?)
                }
                "rust_executable" => targets.push(self.target(TargetKind::RustExecutable)?),
                "rust_library" => targets.push(self.target(TargetKind::RustLibrary)?),
                "d_executable" => targets.push(self.target(TargetKind::DExecutable)?),
                unknown => return Err(Error::Parse(format!("unknown project member '{unknown}'"))),
            }
        }
        if targets.is_empty() {
            return Err(Error::Config(
                "project must define at least one target".to_string(),
            ));
        }
        Ok(Project {
            name,
            version,
            description,
            license,
            edition,
            dependencies,
            targets,
        })
    }

    fn target(&mut self, kind: TargetKind) -> Result<Target> {
        let name = self.string_or_word()?;
        self.expect_symbol('{')?;
        let mut target = Target {
            name,
            kind,
            sources: Vec::new(),
            dependencies: Vec::new(),
            include_dirs: Vec::new(),
            defines: Vec::new(),
            flags: Vec::new(),
            linker_flags: Vec::new(),
            install: false,
        };
        while !self.take_symbol('}') {
            let field = self.word()?;
            self.expect_symbol('=')?;
            match field.as_str() {
                "sources" => target.sources = self.paths()?,
                "dependencies" | "depends" => target.dependencies = self.strings()?,
                "include_dirs" | "includes" => target.include_dirs = self.paths()?,
                "defines" => target.defines = self.strings()?,
                "flags" => target.flags = self.strings()?,
                "linker_flags" => target.linker_flags = self.strings()?,
                "install" => target.install = self.boolean()?,
                unknown => {
                    return Err(Error::Parse(format!("unknown target property '{unknown}'")));
                }
            }
        }
        if target.sources.is_empty() {
            return Err(Error::Config(format!(
                "target '{}' has no sources",
                target.name
            )));
        }
        Ok(target)
    }

    fn paths(&mut self) -> Result<Vec<PathBuf>> {
        let values = self.strings_or_glob()?;
        Ok(values
            .into_iter()
            .map(|value| self.root.join(value))
            .collect())
    }

    fn strings_or_glob(&mut self) -> Result<Vec<String>> {
        if self.take_word("glob") {
            self.expect_symbol('(')?;
            let pattern = self.string_or_word()?;
            self.expect_symbol(')')?;
            return expand_glob(self.root, &pattern);
        }
        self.strings()
    }

    fn strings(&mut self) -> Result<Vec<String>> {
        if let Some(Token::Word(name)) = self.tokens.get(self.position) {
            if self.bindings.contains_key(name) {
                self.position += 1;
                return self.value_as_strings(name);
            }
        }
        self.expect_symbol('[')?;
        let mut values = Vec::new();
        while !self.take_symbol(']') {
            values.push(self.string_or_word()?);
            let _ = self.take_symbol(',');
        }
        Ok(values)
    }

    fn boolean(&mut self) -> Result<bool> {
        let value = self.word()?;
        if self.bindings.contains_key(&value) {
            return self.value_as_bool(&value);
        }
        match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            value => Err(Error::Parse(format!("expected boolean, got '{value}'"))),
        }
    }

    fn string_or_word(&mut self) -> Result<String> {
        match self.next() {
            Some(Token::String(value)) => Ok(value),
            Some(Token::Word(value)) => {
                if self.bindings.contains_key(&value) {
                    self.value_as_string(&value)
                } else {
                    Ok(value)
                }
            }
            _ => Err(Error::Parse("expected a string or identifier".to_string())),
        }
    }

    fn binding(&mut self) -> Result<()> {
        let name = self.word()?;
        if self.bindings.contains_key(&name) {
            return Err(Error::Config(format!(
                "binding '{name}' is declared more than once"
            )));
        }
        self.expect_symbol('=')?;
        let value = self.value()?;
        self.bindings.insert(name, value);
        Ok(())
    }

    fn value(&mut self) -> Result<Value> {
        match self.next() {
            Some(Token::String(value)) => Ok(Value::String(value)),
            Some(Token::Word(value)) => {
                if value == "true" {
                    Ok(Value::Bool(true))
                } else if value == "false" {
                    Ok(Value::Bool(false))
                } else if self.bindings.contains_key(&value) {
                    Ok(Value::Reference(value))
                } else {
                    Ok(Value::String(value))
                }
            }
            Some(Token::Symbol('[')) => {
                let mut values = Vec::new();
                while !self.take_symbol(']') {
                    values.push(self.value()?);
                    let _ = self.take_symbol(',');
                }
                Ok(Value::List(values))
            }
            _ => Err(Error::Parse("expected a binding value".to_string())),
        }
    }

    fn resolve(&self, value: &Value) -> Result<Value> {
        match value {
            Value::Reference(name) => self
                .bindings
                .get(name)
                .ok_or_else(|| Error::Config(format!("binding '{name}' is not defined")))
                .and_then(|value| self.resolve(value)),
            Value::List(values) => Ok(Value::List(
                values
                    .iter()
                    .map(|value| self.resolve(value))
                    .collect::<Result<Vec<_>>>()?,
            )),
            value => Ok(value.clone()),
        }
    }

    fn value_as_string(&self, name: &str) -> Result<String> {
        match self.resolve(
            self.bindings
                .get(name)
                .ok_or_else(|| Error::Config(format!("binding '{name}' is not defined")))?,
        )? {
            Value::String(value) => Ok(value),
            _ => Err(Error::Config(format!("binding '{name}' must be a string"))),
        }
    }

    fn value_as_bool(&self, name: &str) -> Result<bool> {
        match self.resolve(
            self.bindings
                .get(name)
                .ok_or_else(|| Error::Config(format!("binding '{name}' is not defined")))?,
        )? {
            Value::Bool(value) => Ok(value),
            _ => Err(Error::Config(format!("binding '{name}' must be a boolean"))),
        }
    }

    fn value_as_strings(&self, name: &str) -> Result<Vec<String>> {
        match self.resolve(
            self.bindings
                .get(name)
                .ok_or_else(|| Error::Config(format!("binding '{name}' is not defined")))?,
        )? {
            Value::List(values) => values
                .into_iter()
                .map(|value| match value {
                    Value::String(value) => Ok(value),
                    _ => Err(Error::Config(format!(
                        "binding '{name}' must contain only strings"
                    ))),
                })
                .collect(),
            _ => Err(Error::Config(format!("binding '{name}' must be a list"))),
        }
    }

    fn string_or_file(&mut self) -> Result<String> {
        if self.take_word("file") {
            self.expect_symbol('(')?;
            let relative = self.string_or_word()?;
            self.expect_symbol(')')?;
            let path = self.root.join(&relative);
            let value = fs::read_to_string(&path).map_err(|error| {
                Error::Config(format!(
                    "could not read version file '{}': {error}",
                    path.display()
                ))
            })?;
            let value = value.trim();
            if value.is_empty() {
                return Err(Error::Config(format!(
                    "version file '{}' is empty",
                    path.display()
                )));
            }
            return Ok(value.to_string());
        }
        self.string_or_word()
    }

    fn word(&mut self) -> Result<String> {
        match self.next() {
            Some(Token::Word(value)) => Ok(value),
            _ => Err(Error::Parse("expected an identifier".to_string())),
        }
    }

    fn expect_word(&mut self, expected: &str) -> Result<()> {
        let actual = self.word()?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::Parse(format!(
                "expected '{expected}', got '{actual}'"
            )))
        }
    }
    fn take_word(&mut self, expected: &str) -> bool {
        matches!(self.tokens.get(self.position), Some(Token::Word(value)) if value == expected) && {
            self.position += 1;
            true
        }
    }
    fn expect_symbol(&mut self, expected: char) -> Result<()> {
        if self.take_symbol(expected) {
            Ok(())
        } else {
            Err(Error::Parse(format!("expected '{expected}'")))
        }
    }
    fn take_symbol(&mut self, expected: char) -> bool {
        matches!(self.tokens.get(self.position), Some(Token::Symbol(value)) if *value == expected)
            && {
                self.position += 1;
                true
            }
    }
    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += usize::from(token.is_some());
        token
    }
}

fn expand_glob(root: &Path, pattern: &str) -> Result<Vec<String>> {
    let (directory, prefix, suffix) = pattern
        .split_once('*')
        .map(|(left, right)| (Path::new(left), left, right))
        .ok_or_else(|| Error::Parse("glob requires a '*' pattern".to_string()))?;
    let absolute = root.join(directory);
    let mut results = Vec::new();
    for entry in fs::read_dir(&absolute)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(prefix.rsplit('/').next().unwrap_or(prefix))
                        && name.ends_with(suffix)
                })
        {
            results.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    results.sort();
    Ok(results)
}
