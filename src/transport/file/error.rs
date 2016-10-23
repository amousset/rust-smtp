//! Error and result type for file transport

use self::Error::*;
use std::error::Error as StdError;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;

/// An enum of all error kinds.
#[derive(Debug)]
pub enum Error {
    /// Internal client error
    Client(&'static str),
    /// IO error
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Client(_) => "an unknown error occured",
            Io(_) => "an I/O error occured",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Io(ref err) => Some(&*err as &StdError),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Io(err)
    }
}

impl From<&'static str> for Error {
    fn from(string: &'static str) -> Error {
        Client(string)
    }
}

/// SMTP result type
pub type FileResult = Result<(), Error>;
