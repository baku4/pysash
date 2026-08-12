use super::Range;

/// [`SessionHistory`](crate::SessionHistory)의 상태가 입력 소스의 super-set이
/// 되도록 하는 실행 순서.
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
    /// 세션이 지금 어떤 상태인가. statement 하나에 붙는 것은 [`StatementPlan`]에 있다.
    pub diagnostics: Vec<SessionDiagnostic>,
}

/// statement 하나에 대한 판정.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatementPlan {
    /// 입력 소스에서 몇 번째 statement인가.
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
    pub diagnostics: Vec<StatementDiagnostic>,
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
    /// 실행도 그 결과를 건드리지 않았다. 판정에서 `Reuse`를 낳는 유일한 이유다.
    /// [`downgrade_from`](AlignmentPlan::downgrade_from) 뒤에는 `Run`에 남아
    /// "판정은 재사용 가능이었다"를 기록한다.
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

/// 세션이 지금 어떤 상태인가에 대한 주석. plan 전체에 붙는다.
///
/// 에러가 아니다 — 애매한 것은 이미 Run으로 떨어졌으므로 이게 붙어도 plan은
/// 유효하다. 내가 못 본 것과 내가 가정한 것을 드러낸다.
///
/// "세션이 이 소스의 prefix를 넘어 실행했는가"는 여기 없다.
/// [`PlanSummary::residue_len`]이 그 사실 자체이고, 진단으로 한 번 더 말하면
/// 같은 것을 두 군데서 관리하게 된다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SessionDiagnostic {
    /// 실현 밖 실행에 반사적 구문이 있다 — 무엇이 오염됐는지 알 수 없어 전부
    /// Run이다. 반사적 구문이 여럿이면 전부 실린다.
    ///
    /// 이 실행은 입력 소스가 아니라 **세션이 과거에 받은 소스**에 있다. 그래서
    /// `source`([`SessionHistory::sources`](crate::SessionHistory::sources)의
    /// 인덱스)와 `range`(그 소스의 바이트열 기준)가 함께 있어야 위치를 짚을 수 있다.
    OpaqueResidue { source: usize, range: Range },
}

/// statement 하나에 대한 주석.
///
/// 위치는 담지 않는다 — [`StatementPlan::range`]가 이미 그 statement의 위치다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StatementDiagnostic {
    /// 소스가 읽는 이름을 소스 안에서 찾을 수 없다. 세션에 있어야 실행되는
    /// 조각이라는 뜻이고, fresh run에서는 재현되지 않는다.
    UnresolvedReference { name: Box<str> },
}

/// 이 statement가 무엇을 하는가의 분류. 판정이 아니라 사실이다.
///
/// 재사용 판정의 게이트가 아니다 — 판정은 오염 집합이 한다. 이 분류는 호출자가
/// plan을 후처리할 때 쓴다. 예를 들어 외부 파일이 바뀌었을 수 있으니
/// [`ExternalRead`](Effect::ExternalRead)는 재사용하지 않겠다는 정책은 호출자의
/// 몫이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// 이름 바인딩 외에 관측 가능한 효과 없음.
    Pure,
    /// 모듈 임포트. `sys.modules` 캐시 덕에 재실행이 멱등이다.
    Import,
    /// print / display / 로깅 같은 출력.
    Output,
    /// 파일·네트워크·`input()` — 외부 세계를 읽는다.
    ExternalRead,
    /// 파일 쓰기·네트워크 전송·subprocess — 외부 세계를 바꾼다.
    ExternalWrite,
    /// random / time / uuid 같은 비결정적 값 생성.
    Nondeterministic,
    /// 반사적 구문 — 무엇이든 할 수 있다.
    Opaque,
}

impl AlignmentPlan {
    /// 다시 실행해야 하는 것만. 입력 소스의 순서 그대로다.
    pub fn run_plans(&self) -> impl Iterator<Item = &StatementPlan> {
        self.plans.iter().filter(|plan| plan.action == Action::Run)
    }

    /// `index`부터 끝까지 전부 `Run`으로 내린다.
    ///
    /// 외부 세계가 바뀌었을 수 있어 (예: [`Effect::ExternalRead`]) 판정보다 더
    /// 실행하고 싶을 때 쓴다. 한 지점만 내리고 그 아래를 재사용하면 상태가
    /// 어긋나므로, 지점 이후 전체를 내리는 것만 제공한다. Reuse → Run 방향만
    /// 존재하므로 correctness를 깰 수 없다. 내려간 step의 `reason`은 판정
    /// 당시의 것이 그대로 남는다.
    pub fn downgrade_from(&mut self, index: usize) {
        for plan in &mut self.plans {
            if plan.index >= index && plan.action == Action::Reuse {
                plan.action = Action::Run;
                plan.witness = None;
            }
        }
        let run = self
            .plans
            .iter()
            .filter(|plan| plan.action == Action::Run)
            .count();
        self.summary.run = run;
        self.summary.reused = self.summary.total - run;
        self.summary.first_run = self
            .plans
            .iter()
            .find(|plan| plan.action == Action::Run)
            .map(|plan| plan.index);
    }
}
