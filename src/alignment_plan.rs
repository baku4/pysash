use super::{Action, DecisionReason, Range};

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
}

/// statement 하나에 대한 판정.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StatementPlan {
    /// 입력 소스 `statements()`에서의 인덱스.
    pub index: usize,
    /// 입력 소스 원본 바이트열에서의 위치.
    pub range: Range,
    pub action: Action,
    pub reason: DecisionReason,
}

impl AlignmentPlan {
    /// 다시 실행해야 하는 것만. 입력 소스의 순서 그대로이고, 언제나 소스의 뒤쪽
    /// 연속 구간이다 — 재사용은 세션의 꼬리에 이어 붙는 앞부분에서만 나오기 때문이다.
    pub fn run_plans(&self) -> impl Iterator<Item = &StatementPlan> {
        self.plans.iter().filter(|plan| plan.action == Action::Run)
    }
}
