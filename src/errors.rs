use std::io;
use std::fmt;

use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

#[derive(Clone, Copy, Debug, Eq, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(display = "Unexpected end of input")]
    EndOfInput,
    #[fail(display = "Unclosed `\\verbatim` command (started at line {}, column {})", _0, _1)]
    UnclosedVerbatim(usize, usize),
    #[fail(display = "Invalid UTF-8 in line {}", _0)]
    Unicode(usize),
    #[fail(display = "An IO error occurred while reading line {}", _0)]
    Io(usize),
}

impl ErrorKind {
    pub fn from_io(err: &io::Error, line: usize) -> ErrorKind {
        match err.kind() {
            io::ErrorKind::InvalidData => ErrorKind::Unicode(line),
            _ => ErrorKind::Io(line),
        }
    }
}
