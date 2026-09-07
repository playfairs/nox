use crate::error::{Error, Result};
use crate::model::{Project, Target, TargetKind};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
enum Token {
  Word(String),
  String(String),
  Symbol(char),
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
}

impl<'a> Parser<'a> {
  fn project(&mut self) -> Result<Project> {
    self.expect_word("project")?;
    let name = self.string_or_word()?;
    self.expect_symbol('{')?;
    let mut version = "0.1.0".to_string();
    let mut targets = Vec::new();
    while !self.take_symbol('}') {
      match self.word()?.as_str() {
        "version" => {
          self.expect_symbol('=')?;
          version = self.string_or_word()?;
        }
        "executable" => targets.push(self.target(TargetKind::Executable)?),
        "static_library" | "static" => targets.push(self.target(TargetKind::StaticLibrary)?),
        "shared_library" | "shared" => targets.push(self.target(TargetKind::SharedLibrary)?),
        "rust_executable" => targets.push(self.target(TargetKind::RustExecutable)?),
        "rust_library" => targets.push(self.target(TargetKind::RustLibrary)?),
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
        unknown => return Err(Error::Parse(format!("unknown target property '{unknown}'"))),
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
    Ok(
      values
        .into_iter()
        .map(|value| self.root.join(value))
        .collect(),
    )
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
    self.expect_symbol('[')?;
    let mut values = Vec::new();
    while !self.take_symbol(']') {
      values.push(self.string_or_word()?);
      let _ = self.take_symbol(',');
    }
    Ok(values)
  }

  fn boolean(&mut self) -> Result<bool> {
    match self.word()?.as_str() {
      "true" => Ok(true),
      "false" => Ok(false),
      value => Err(Error::Parse(format!("expected boolean, got '{value}'"))),
    }
  }

  fn string_or_word(&mut self) -> Result<String> {
    match self.next() {
      Some(Token::String(value) | Token::Word(value)) => Ok(value),
      _ => Err(Error::Parse("expected a string or identifier".to_string())),
    }
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
    matches!(self.tokens.get(self.position), Some(Token::Symbol(value)) if *value == expected) && {
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
          name.starts_with(prefix.rsplit('/').next().unwrap_or(prefix)) && name.ends_with(suffix)
        })
    {
      results.push(
        path
          .strip_prefix(root)
          .unwrap_or(&path)
          .to_string_lossy()
          .replace('\\', "/"),
      );
    }
  }
  results.sort();
  Ok(results)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn parses_targets() {
    let project = parse(
      "project \"demo\" { executable \"app\" { sources = [\"main.c\"] flags = [\"-Wall\"] } }",
      Path::new("."),
    )
    .unwrap();
    assert_eq!(project.targets[0].name, "app");
    assert_eq!(project.targets[0].flags, vec!["-Wall"]);
  }
}
