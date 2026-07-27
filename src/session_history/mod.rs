use super::Statement;

/// 지금까지 실행된 것의 선형 기록.
///
/// Python이나 IPython REPL에 입력하듯 `PythonSource`를 순서대로 밀어 넣는다.
/// **들어오는 소스는 성공한 실행이어야 한다** — 검증하지 않는 계약이다.
///
/// jupyter처럼 위아래를 오간 기록도 아니고 marimo처럼 선언 순서가 무관한 모델도
/// 아니다. 순서가 곧 의미이므로, 순서가 바뀌면 재사용도 없다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SessionHistory {
    /// 지금 "실현된" 것으로 세는 선형 실행 열.
    realized: Vec<Statement>,
    /// 실현 열 밖에서 실행되어 효과만 남은 것들. 순수 push 워크플로에서는 항상
    /// 비어 있다.
    residue: Vec<Statement>,
}

mod prefix;

mod record;
mod align;
