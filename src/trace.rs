use super::source::PythonSource;
use super::statement::Statement;

/// A statement execution identified by source position and monotonic session sequence.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecRef {
    pub source: PythonSource,
    pub index: usize,
    /// Global execution order, which remains fixed when the realized sequence changes.
    pub seq: usize,
}

impl ExecRef {
    /// Returns the executed statement.
    pub fn statement(&self) -> &Statement {
        &self.source.statements()[self.index]
    }

    /// Returns the executed statement's original bytes.
    pub fn text(&self) -> &[u8] {
        self.source.slice(self.statement().range)
    }
}
