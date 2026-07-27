/// statement 하나를 어떻게 할 것인가. 두 갈래뿐이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// 세션의 그 실행을 이 소스의 실행으로 센다. 다시 돌리지 않는다.
    Reuse,
    /// 다시 실행한다.
    Run,
}

/// [`Action`]을 그렇게 정한 이유.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DecisionReason {
    /// 세션이 이 지점까지 이 소스와 canonical하게 동일하고, 그 뒤로 아무것도 더
    /// 실행하지 않았다. `Reuse`를 낳는 유일한 이유다.
    ReusableExecution,
    /// 세션과 이 소스가 갈라지는 편집 지점이다.
    StatementChanged,
    /// 세션에 대응하는 실행이 없다.
    NoMatchingExecution,
    /// 세션이 이 소스 밖에서 더 실행한 것이 있어, 이 statement가 만든 것이
    /// 그대로인지 알 수 없다.
    DependencyChanged,
}
