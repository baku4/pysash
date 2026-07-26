use std::error::Error;
// module-rule: allow import-alias -- name-conflict: avoid collision with core::result::Result
use std::fmt::{Display, Formatter, Result as FmtResult};
use super::Range;

/// 소스를 `PythonSource`로 만들지 못한 이유.
///
/// 이 crate에서 `Err`가 되는 유일한 것이다. 파싱이 깨지면 statement가 하나도 없어
/// 정렬할 대상 자체가 없기 때문이다. 그 밖의 모든 "못 봤다 / 가정했다"는
/// [`Diagnostic`](super::Diagnostic)으로 보고하며 plan 생성을 막지 않는다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParseError {
    pub range: Range,
    pub kind: ParseErrorKind,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseErrorKind {
    /// 문법 오류. 실행이 실패하지 않음을 보장하는 것은 아니다 — 파싱 가능성만 본다.
    Syntax { message: Box<str> },
    /// 입력 바이트열이 UTF-8이 아니다.
    NotUtf8 { offset: u32 },
    /// PEP 263 인코딩 선언이 UTF-8이 아니다. `Range`가 어느 바이트열 기준인지
    /// 흐려지므로 v0.1은 명시적으로 거부한다.
    UnsupportedEncoding { declared: Box<str> },
    /// 소스가 너무 길다. offset이 `u32`이므로 4 GiB가 상한이다.
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
