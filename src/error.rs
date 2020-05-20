use std::error::Error as StdError;
use std::fmt;
use std::io;
use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Http(hyper::Error),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            Error::Http(_) => "HTTP handshake error",
            Error::Io(_) => "IO error",
        };
        if let Some(cause) = self.source() {
            write!(f, "{}: {}", description, cause)
        } else {
            f.write_str(description)
        }
    }
}

impl StdError for Error {}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::Http(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<JoinError> for Error {
    fn from(err: JoinError) -> Self {
        Self::from(io::Error::from(err))
    }
}
