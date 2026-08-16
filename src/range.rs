/// A half-open UTF-8 byte range in the original [`PythonSource`](crate::source::PythonSource).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

impl Range {
    /// Creates the half-open range `[start, end)`.
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}
