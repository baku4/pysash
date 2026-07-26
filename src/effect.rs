/// statement가 무엇을 하는가의 분류. 판정이 아니라 사실이다.
///
/// 실현(realization) 의미론에서 `Effect`는 **재사용 건전성의 게이트가 아니다** —
/// 재사용된 실행은 이미 이 소스의 실행이므로 `print`는 제때 출력됐고 `random()`은
/// 그 값이 맞다. 이 분류는 진단과 호출자의 후처리를 위한 것이다. 외부 세계가
/// 그사이 바뀌었을 수 있다고 보는 호출자는 `ExternalRead` step을 직접 Run으로
/// 내릴 수 있다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Effect {
    /// 이름 바인딩 외에 관측 가능한 효과가 없다.
    #[default]
    Pure,
    /// 모듈 임포트. `sys.modules` 캐시 때문에 재실행이 멱등이다.
    Import,
    /// `print` / display / 로깅.
    Output,
    /// 파일·네트워크·`input()` — 외부 세계를 읽는다.
    ExternalRead,
    /// 파일 쓰기·네트워크 POST·`subprocess`.
    ExternalWrite,
    /// `random` / `time` / `uuid` / `id`.
    Nondeterministic,
    /// 반사적 구문. 무엇이든 할 수 있다.
    Opaque,
}
