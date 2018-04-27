use std::io::BufRead;

use failure::Error;
use failure::err_msg;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    /// `{`
    BeginGroup,
    /// `}`
    EndGroup,
    /// A single character, including escaped `\{`, `\}`, and `\\`.
    Char(char),
    /// A command to be executed.
    Command(String),
    /// A string to be included verbatim in the output without further parsing.
    Verbatim(String),
}

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
        self.str_buf.clear();
        self.input.read_line(&mut self.str_buf)?;
        self.vec_buf = self.str_buf.chars().collect();
        self.column = 0;
        self.line += 1;
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
        self.next()?.ok_or(err_msg("Unexpected end of input"))
    }

    /// Returns the next character in the line without advancing the stream. A `None` value just
    /// indicates that the end of the line has been reached, not necessarily the end of the text.
    pub fn peek(&mut self) -> Option<&char> {
        self.vec_buf.get(self.column + 1)
    }
}

/// An `Iterator` that produces the tokens found in a `BufRead`.
#[derive(Debug)]
pub struct Tokens<R> {
    input: BufReadIter<R>,
}

impl<R: BufRead> Tokens<R> {
    /// Constructs a new `Tokens` from the given `BufRead`.
    pub fn new(input: R) -> Tokens<R> {
        Tokens {
            input: BufReadIter::new(input),
        }
    }

    /// Advances the stream, returning the next token if present, or any errors encountered.
    pub fn next(&mut self) -> Result<Option<Token>, Error> {
        use self::Token::*;
        Ok(match self.input.next()? {
            Some('{') => Some(BeginGroup),
            Some('}') => Some(EndGroup),
            Some('\\') => match self.input.expect_next()? {
                '\\' => Some(Char('\\')),
                '{' => Some(Char('{')),
                '}' => Some(Char('}')),
                c if c.is_alphanumeric() => {
                    let command = self.ident(c);
                    if command == "verbatim" {
                        let delim = self.input.expect_next()?;
                        let line = self.input.line();
                        let column = self.input.column();
                        Some(Verbatim(self.verbatim(delim, line, column)?))
                    } else {
                        Some(Command(command))
                    }
                }
                c => Some(Command(c.to_string())),
            },
            Some(c) => Some(Char(c)),
            None => None,
        })
    }

    /// Extracts an identifier from the input stream, starting with the given `char`.
    fn ident(&mut self, first: char) -> String {
        let mut command = first.to_string();
        while let Some(&c) = self.input.peek() {
            if c.is_alphanumeric() {
                // we can unwrap here, because errors can only occur in `input.next()` when reading
                // a new line, but since we already matched `input.peek()`, we know we won't be
                // refilling the buffer.
                self.input.next().unwrap();
                command.push(c);
            } else {
                break;
            }
        }
        command
    }

    /// Extracts a verbatim string from the input stream, using the given delimiter.
    ///
    /// Within a verbatim string, the delimiter can be escaped with itself.
    fn verbatim(
        &mut self,
        delimiter: char,
        start_line: usize,
        start_column: usize,
    ) -> Result<String, Error> {
        let mut verb = String::new();
        loop {
            // we can use `next` here because we will consume the closing delimiter.
            match self.input.next()? {
                Some(c) if c == delimiter => match self.input.peek() {
                    Some(&c) if c == delimiter => {
                        self.input.next()?;
                        verb.push(delimiter);
                    }
                    Some(_) | None => break,
                },
                Some(c) => verb.push(c),
                None => Err(format_err!(
                    "Unclosed `\\verbatim` command (started at line {}, column {})",
                    start_line,
                    start_column
                ))?,
            }
        }
        Ok(verb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let input = "{ab}".as_bytes();
        let mut tokens = Tokens::new(input);
        let mut output = Vec::new();
        while let Ok(Some(tok)) = tokens.next() {
            output.push(tok);
        }
        assert_eq!(
            output,
            vec![
                Token::BeginGroup,
                Token::Char('a'),
                Token::Char('b'),
                Token::EndGroup,
            ]
        );
        assert!(tokens.next().unwrap().is_none());
    }

    #[test]
    fn command() {
        let input = "\\abc".as_bytes();
        let mut tokens = Tokens::new(input);
        let mut output = Vec::new();
        while let Ok(Some(tok)) = tokens.next() {
            output.push(tok);
        }
        assert_eq!(output, vec![Token::Command(String::from("abc"))]);
        assert!(tokens.next().unwrap().is_none());
    }

    #[test]
    fn verbatim() {
        let input = "\\verbatim!a\\b!".as_bytes();
        let mut tokens = Tokens::new(input);
        let mut output = Vec::new();
        while let Ok(Some(tok)) = tokens.next() {
            output.push(tok);
        }
        assert_eq!(output, vec![Token::Verbatim(String::from("a\\b"))]);
        assert!(tokens.next().unwrap().is_none());
    }

    #[test]
    fn verbatim_unclosed() {
        let input = "\\verbatim!a\\b".as_bytes();
        let mut tokens = Tokens::new(input);
        let err = tokens.next().unwrap_err();
        assert_eq!(
            format!("{}", err),
            "Unclosed `\\verbatim` command (started at line 1, column 9)"
        );
    }

    #[test]
    fn verbatim_escape() {
        let input = "\\verbatim!a!!b!".as_bytes();
        let mut tokens = Tokens::new(input);
        let mut output = Vec::new();
        while let Ok(Some(tok)) = tokens.next() {
            output.push(tok);
        }
        assert_eq!(output, vec![Token::Verbatim(String::from("a!b"))]);
        assert!(tokens.next().unwrap().is_none());
    }
}
