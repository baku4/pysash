use std::sync::Arc;
use super::statement::Statement;

/// Syntactically valid Python source with preserved input bytes and top-level statements.
///
/// Parsing does not guarantee successful execution. Clones share their storage.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PythonSource {
    raw: Arc<[u8]>,
    statements: Arc<[Statement]>,
}

mod parse_error;
pub use parse_error::{ParseError, ParseErrorKind};

mod parse;
mod access;
