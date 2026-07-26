use crate::diagnostic::Diagnostic;
use crate::range::Range;
use crate::source_mode::SourceMode;
use crate::statement::Statement;
use super::PythonSource;

impl PythonSource {
    /// 생성에 쓰인 원본 바이트열. statement의 `range`는 이 바이트열 기준이다.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    pub fn mode(&self) -> SourceMode {
        self.mode
    }

    /// 소스에 나타난 순서 그대로의 top-level statement 목록.
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// `range`가 가리키는 원본 바이트. 범위를 벗어나면 잘려 나간다.
    pub fn slice(&self, range: Range) -> &[u8] {
        let len = self.raw.len();
        let start = (range.start as usize).min(len);
        let end = (range.end as usize).clamp(start, len);
        &self.raw[start..end]
    }
}
