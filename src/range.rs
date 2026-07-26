/// raw bytes 안의 반열린 구간 `[start, end)`. UTF-8 byte offset이다.
///
/// `PythonSource`가 기억하는 원본 바이트열 기준이며, 이 구간을 잘라내 다시 파싱하면
/// 같은 statement가 나온다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

impl Range {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    pub fn contains(&self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }
}
