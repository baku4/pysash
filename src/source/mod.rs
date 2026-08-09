use std::sync::Arc;
use super::statement::Statement;

/// 파싱된 Python 소스.
///
/// **이 값이 존재한다는 것은 문법적으로 파싱 가능한 Python이라는 뜻이다.** 다른
/// 상태는 표현할 수 없다. 다만 파싱에 성공했다는 것이 실행에 실패하지 않는다는
/// 보장은 아니다.
///
/// 원본 바이트열을 그대로 기억해 두므로 각 statement의 `range`로 원문을 잘라낼 수
/// 있다. clone은 전부 `Arc` 백업이라 O(1)이다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PythonSource {
    raw: Arc<[u8]>,
    statements: Arc<[Statement]>,
}

mod parse_error;
pub use parse_error::{ParseError, ParseErrorKind};

mod parse;
mod access;
