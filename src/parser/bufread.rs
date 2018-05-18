use std::io::BufRead;

use failure::{Backtrace, Context, Fail, ResultExt};

use errors::{Error, ErrorKind};

/// A struct providing `next` and `peek` methods to iterate over the chars of a `BufRead`.
///
/// This struct is not actually an `Iterator`, because `next` returns `Result<Option<char>,
/// Error>`, instead of `Option<_>`.
#[derive(Debug)]
pub struct BufReadIter<R> {
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
