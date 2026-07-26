/// statement 하나를 어떻게 할 것인가. 두 갈래뿐이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// 세션의 그 실행을 이 소스의 실행으로 센다. 다시 돌리지 않는다.
    Reuse,
    /// 다시 실행한다.
    Run,
}

/// [`Action`]을 그렇게 정한 이유. 다섯 갈래가 상호배타적이고 완전한 결정 트리를 이룬다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DecisionReason {
    /// 세션의 앞부분이 이 지점까지 canonical하게 동일하고, 그 효과를 훼손한 것이 없다.
    /// `Reuse`를 낳는 유일한 이유다.
    ReusableExecution,
    /// 세션에 대응하는 실행이 없다.
    NoMatchingExecution,
    /// 세션과 이 소스가 갈라지는 편집 지점이다.
    StatementChanged,
    /// 이 statement가 만드는 이름이 잉여 실행에 오염됐다.
    DependencyChanged { name: Box<str> },
    /// 이 statement가 바인딩하는 이름이 잉여 실행에서 다시 바인딩됐다.
    BindingChanged { name: Box<str> },
}
