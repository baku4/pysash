use super::{Action, DecisionReason, Range};

/// `SessionHistory`의 상태가 입력 소스의 super-set이 되도록 하는 실행 순서.
///
/// 입력 소스의 statement 하나하나에 대해 재사용할지 다시 실행할지를 순서대로 담는다.
/// `Run`인 step을 이 순서대로 실행하면 소스 전체를 실행한 것과 같은 상태가 된다.
///
/// 결과 리포트이므로 전부 열려 있다 — 지킬 불변식이 없고, plan을 세션에 되먹이는
/// 경로도 없다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AlignmentPlan {
    pub steps: Vec<Step>,
}

/// statement 하나에 대한 판정.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// 입력 소스 `statements()`에서의 인덱스.
    pub index: usize,
    /// 입력 소스 원본 바이트열에서의 위치.
    pub range: Range,
    pub action: Action,
    pub reason: DecisionReason,
}

impl AlignmentPlan {
    /// 아무것도 다시 실행할 필요가 없다.
    pub fn is_full_reuse(&self) -> bool {
        self.steps.iter().all(|step| step.action == Action::Reuse)
    }

    /// 다시 실행해야 하는 step만. 입력 소스의 순서 그대로다.
    pub fn run_steps(&self) -> impl Iterator<Item = &Step> {
        self.steps.iter().filter(|step| step.action == Action::Run)
    }

    pub fn reused_count(&self) -> usize {
        self.steps.len() - self.run_steps().count()
    }
}
