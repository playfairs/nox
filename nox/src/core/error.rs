use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(String),
    Config(String),
    Process(String),
    Exit(i32),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Parse(message) => write!(formatter, "parse error: {message}"),
            Self::Config(message) => write!(formatter, "configuration error: {message}"),
            Self::Process(message) => write!(formatter, "process error: {message}"),
            Self::Exit(code) => write!(formatter, "process exited with status {code}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
