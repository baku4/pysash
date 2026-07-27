use super::python_source::PythonSource;

/// 지금까지 성공적으로 실행된 것의 선형 기록.
///
/// Python이나 IPython REPL에 입력하듯 `PythonSource`를 순서대로 밀어 넣는다.
/// **들어오는 소스는 성공한 실행이어야 한다** — 검증하지 않는 계약이다.
///
/// linear하다. jupyter처럼 위아래를 오간 기록도 아니고 marimo처럼 선언 순서가
/// 무관한 모델도 아니다. 순서가 곧 의미다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SessionHistory {
    /// 입력된 소스가 순서대로. 각 소스는 자기 statement를 들고 있으므로 이 하나가
    /// 곧 `SessionHistory → PythonSource → statement` 트리다.
    ///
    /// statement만 떼어 이어 붙이면 "어느 소스의 몇 번째"가 사라지고, 원문을
    /// 잘라볼 대상도 함께 사라진다. `PythonSource`는 전부 `Arc` 백업이라 보관
    /// 비용이 O(1)이다.
    sources: Vec<PythonSource>,
}

mod overlap;

mod record;
mod align;
