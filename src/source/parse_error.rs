use std::error::Error;
// module-rule: allow import-alias -- name-conflict: avoid collision with core::result::Result
use std::fmt::{Display, Formatter, Result as FmtResult};
use crate::Range;

/// A failure to construct [`PythonSource`](super::PythonSource) from input bytes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseError {
    pub range: Range,
    pub kind: ParseErrorKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseErrorKind {
    /// Python syntax error reported by the parser.
    Syntax { message: Box<str> },
    /// Input is not valid UTF-8.
    NotUtf8 { offset: u32 },
    /// A PEP 263 cookie declares an unsupported non-UTF-8 encoding.
    UnsupportedEncoding { declared: Box<str> },
    /// Input exceeds the `u32` byte-offset limit.
    TooLarge { len: usize },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "byte {}..{}: ", self.range.start, self.range.end)?;
        match &self.kind {
            ParseErrorKind::Syntax { message } => write!(f, "syntax error: {message}"),
            ParseErrorKind::NotUtf8 { offset } => write!(f, "not valid UTF-8 at byte {offset}"),
            ParseErrorKind::UnsupportedEncoding { declared } => {
                write!(
                    f,
                    "unsupported source encoding `{declared}`, expected utf-8"
                )
            }
            ParseErrorKind::TooLarge { len } => {
                write!(f, "source of {len} bytes exceeds the u32 offset limit")
            }
        }
    }
}

impl Error for ParseError {}
