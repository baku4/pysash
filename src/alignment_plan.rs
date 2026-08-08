use super::{Diagnostic, Effect, Range};

/// `SessionHistory`의 상태가 입력 소스의 super-set이 되도록 하는 실행 순서.
///
/// 입력 소스의 statement 하나하나에 대해 재사용할지 다시 실행할지를 순서대로 담는다.
/// `Run`인 것을 이 순서대로 실행하면 소스 전체를 실행한 것과 같은 상태가 된다.
///
/// 결과 리포트이므로 전부 열려 있다 — 지킬 불변식이 없고, 이걸 세션에 되먹이는
/// 경로도 없다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AlignmentPlan {
    pub plans: Vec<StatementPlan>,
    pub summary: PlanSummary,
    /// plan 전체에 대한 주석. statement 하나에 붙는 것은 [`StatementPlan`]에 있다.
    pub diagnostics: Vec<Diagnostic>,
}

/// statement 하나에 대한 판정.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatementPlan {
    /// 입력 소스 `statements()`에서의 인덱스.
    pub index: usize,
    /// 입력 소스 원본 바이트열에서의 위치.
    pub range: Range,
    pub action: Action,
    pub reason: DecisionReason,
    /// 이 statement가 무엇을 하는가의 분류. 호출자 후처리용이다.
    pub effect: Effect,
    /// `Reuse`일 때 근거가 된 실행의 세션 내 위치 (0부터, 소스 경계를 무시하고
    /// 이어 붙인 statement 순서). `Run`이면 없다.
    pub witness: Option<usize>,
    pub diagnostics: Vec<Diagnostic>,
}

/// plan의 요약 숫자들.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlanSummary {
    /// 입력 소스의 statement 수.
    pub total: usize,
    /// 그중 `Reuse`.
    pub reused: usize,
    /// 그중 `Run`.
    pub run: usize,
    /// 세션과 입력 소스의 최장 공통 canonical prefix 길이.
    pub prefix_len: usize,
    /// prefix를 넘어 세션이 추가로 실행한 statement 수. 0이면 세션이 이 소스의
    /// 순수 prefix다.
    pub residue_len: usize,
    /// 첫 `Run`의 인덱스. 전부 `Reuse`면 없다.
    pub first_run: Option<usize>,
}

/// statement 하나를 어떻게 할 것인가. 두 갈래뿐이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// 이 statement의 기존 실행을 그대로 쓴다. 다시 돌리지 않는다.
    Reuse,
    /// 다시 실행한다.
    Run,
}

/// [`Action`]을 그렇게 정한 이유.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum DecisionReason {
    /// 세션이 이 statement를 이 자리에서 이미 실행했고, 그 뒤에 일어난 어떤
    /// 실행도 그 결과를 건드리지 않았다. `Reuse`를 낳는 유일한 이유다.
    ReusableExecution,
    /// 세션이 이 자리에 다른 statement를 실행했다. 편집 지점이다.
    StatementChanged,
    /// 재사용의 근거로 삼을 수 있는 실행이 없다 — 세션이 이 statement를 이
    /// 자리에서 실행한 적이 없거나, 세션 상태를 더는 신뢰할 수 없다.
    NoMatchingExecution,
    /// 세션이 이 statement를 이 자리에서 실행하긴 했지만, 그 뒤의 실행이
    /// 이 statement가 의존하는 `name`을 변경했을 수 있다.
    DependencyChanged { name: Box<str> },
    /// 세션이 이 statement를 이 자리에서 실행하긴 했지만, 그 뒤의 실행이
    /// 이 statement가 바인딩한 `name`을 다시 바인딩했다.
    BindingChanged { name: Box<str> },
}

impl AlignmentPlan {
    /// 다시 실행해야 하는 것만. 입력 소스의 순서 그대로다.
    pub fn run_plans(&self) -> impl Iterator<Item = &StatementPlan> {
        self.plans.iter().filter(|plan| plan.action == Action::Run)
    }
}
