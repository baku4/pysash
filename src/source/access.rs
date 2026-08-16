use crate::range::Range;
use crate::statement::Statement;
use super::PythonSource;

impl PythonSource {
    /// Returns the original input bytes used by statement ranges.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Returns top-level statements in source order.
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    /// Returns the original bytes in `range`, clamped to the input bounds.
    pub fn slice(&self, range: Range) -> &[u8] {
        let len = self.raw.len();
        let start = (range.start as usize).min(len);
        let end = (range.end as usize).clamp(start, len);
        &self.raw[start..end]
    }
}
