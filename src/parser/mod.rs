use failure::{Backtrace, Context, Fail, ResultExt};

use errors::{Error, ErrorKind};
use self::bufread::BufReadIter;

mod bufread;

/// A structure for parsing an input stream
#[derive(Debug)]
pub struct Parser<R> {
    input: BufReadIter<R>,
}
