/// statement 하나를 어떻게 할 것인가. 두 갈래뿐이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// 이 statement의 기존 실행을 그대로 쓴다. 다시 돌리지 않는다.
    Reuse,
    /// 다시 실행한다.
    Run,
}

/// [`Action`]을 그렇게 정한 이유.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DecisionReason {
    /// 세션의 끝이 이 지점까지 이 소스의 앞과 이어진다 — 방금 이 소스를 여기까지
    /// 실행한 것이고 그 뒤에 아무 일도 없었다. `Reuse`를 낳는 유일한 이유다.
    ReusableExecution,
    /// 세션이 이 자리에 다른 statement를 실행했다. 편집 지점이다.
    StatementChanged,
    /// 재사용할 수 있는 실행이 없다 — 즉 세션의 끝에 붙어 있어 지금 상태가 그
    /// 직후인 실행이 없다.
    NoMatchingExecution,
}
