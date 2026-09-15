use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    InvalidDirectory(String),
    UnsupportedLanguage(String),
    Configuration(String),
    Generation(String),
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidDirectory(message) => {
                write!(formatter, "invalid project directory: {message}")
            }
            Self::UnsupportedLanguage(language) => {
                write!(formatter, "unsupported language '{language}'")
            }
            Self::Configuration(message) => write!(formatter, "configuration error: {message}"),
            Self::Generation(message) => write!(formatter, "build generation failed: {message}"),
        }
    }
}

impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl Error {
    pub fn into_core(self) -> crate::core::error::Error {
        crate::core::error::Error::Config(self.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
