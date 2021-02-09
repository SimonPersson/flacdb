use rusqlite::Error as RusqliteError;
use std::convert::From;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Utf8(std::str::Utf8Error),
    MalformedVorbisComment(String),
    InvalidFlacHeader(PathBuf),
    Sqlite(RusqliteError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Error::Io(e) => write!(f, "I/O Error: {}", e),
            Error::Utf8(e) => write!(f, "UTF8 error: {}", e),
            Error::MalformedVorbisComment(e) => write!(f, "Malformed vorbis comment: {}", e),
            Error::InvalidFlacHeader(p) => write!(f, "Invalid flac file: {}", p.display()),
            Error::Sqlite(e) => write!(f, "Sqlite error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Error {
        Error::Utf8(e)
    }
}

impl From<RusqliteError> for Error {
    fn from(e: rusqlite::Error) -> Error {
        Error::Sqlite(e)
    }
}
