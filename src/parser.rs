use std::fmt;
use std::io::{self, BufRead};

use failure::{Backtrace, Context, Fail, ResultExt};

/// A struct providing `next` and `peek` methods to iterate over the chars of a `BufRead`.
///
/// This struct is not actually an `Iterator`, because `next` returns `Result<Option<char>,
/// Error>`, instead of `Option<_>`.
#[derive(Debug)]
struct BufReadIter<R> {
    input: R,
    str_buf: String,
    vec_buf: Vec<char>,
    column: usize,
    line: usize,
}

impl<R: BufRead> BufReadIter<R> {
    /// Constructs a new `BufReadIter` from the given `BufRead`.
    pub fn new(input: R) -> BufReadIter<R> {
        BufReadIter {
            input,
            str_buf: String::new(),
            vec_buf: Vec::new(),
            column: 0,
            line: 0,
        }
    }

    /// Returns the current character number in the line.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns the current line number.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Fills the internal buffer, discarding its old contents.
    fn fill_buffer(&mut self) -> Result<(), Error> {
        self.column = 0;
        self.line += 1;
        self.str_buf.clear();
        self.input
            .read_line(&mut self.str_buf)
            .with_context(|e| ErrorKind::from_io(e, self.line))?;
        self.vec_buf = self.str_buf.chars().collect();
        Ok(())
    }

    /// Advances the iterator, returning the next character if present, or any errors encountered.
    pub fn next(&mut self) -> Result<Option<char>, Error> {
        self.column += 1;
        match self.vec_buf.get(self.column) {
            Some(&c) => Ok(Some(c)),
            None => self.fill_buffer()
                .map(|()| self.vec_buf.get(self.column).map(|&c| c)),
        }
    }

    /// Advances the iterator, returning the next character. If end of input is reached, returns an
    /// error.
    pub fn expect_next(&mut self) -> Result<char, Error> {
        self.next()?.ok_or(ErrorKind::EndOfInput.into())
    }

    /// Returns the next character in the line without advancing the stream. A `None` value just
    /// indicates that the end of the line has been reached, not necessarily the end of the text.
    pub fn peek(&mut self) -> Option<&char> {
        self.vec_buf.get(self.column + 1)
    }
}

/// A structure for parsing an input stream
#[derive(Debug)]
pub struct Parser<R> {
    input: BufReadIter<R>,
}

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
