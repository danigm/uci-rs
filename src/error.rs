use std;
use std::fmt;
use std::convert::From;
use std::io;

/// The error type for any errors encountered with the engine.
#[derive(Debug)]
pub enum EngineError {
    /// Wrapper around any io errors encountered while trying to communicate with the engine.
    Io(io::Error),

    /// Engine doesn't recognize the specified option.
    UnknownOption(String),

    NotFound,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EngineError::Io(ref err) => write!(f, "IO error: {}", err),
            EngineError::UnknownOption(ref option) => write!(f, "No such option: '{}'", option.as_str()),
            EngineError::NotFound => write!(f, "Pattern not found"),
        }
    }
}

impl std::error::Error for EngineError {
    fn description(&self) -> &str {
        match *self {
            EngineError::Io(ref err) => err.description(),
            EngineError::UnknownOption(..) => "Unknown option",
            EngineError::NotFound => "Pattern not found",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            EngineError::Io(ref err) => Some(err),
            EngineError::UnknownOption(..) => None,
            EngineError::NotFound => None,
        }
    }
}

impl From<io::Error> for EngineError {
    fn from(err: io::Error) -> EngineError {
        EngineError::Io(err)
    }
}

/// A Result type which uses [`EngineError`] for representing errors.
///
/// [`EngineError`]: enum.EngineError.html
pub type Result<T> = std::result::Result<T, EngineError>;
