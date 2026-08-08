//! PySASH — Python Source Alignment with Session History.
//!
//! `SessionHistory`와 새로운 `PythonSource`를 비교해, 각 statement의 기존 실행을
//! 재사용할 수 있는지 판단한다. 정적 분석만 수행하며 Python 코드를 실행하지 않는다.
//!
//! 재사용한다는 것은 **각 statement의 기존 실행을 그대로 쓴다**는 뜻이다. 값을
//! 다시 계산해도 같은지를 따지는 것이 아니다.
//!
//! `SessionHistory`는 linear하다 — jupyter처럼 위아래를 오간 기록도 아니고 marimo처럼
//! 선언 순서가 무관한 모델도 아니다. 순서가 곧 의미다.
//!
//! 잘못된 재사용은 조용히 틀린 결과이고 불필요한 재실행은 낭비일 뿐이다. 이 비대칭이
//! 이 crate 전반의 판정 기준이다 — 애매하면 언제나 다시 실행한다.
//!
//! # 쓰는 법
//!
//! ```
//! use pysash::{Action, DecisionReason, SessionHistory};
//! use pysash::python_source::PythonSource;
//!
//! // 1. REPL에 입력하듯, 실제로 성공한 실행만 순서대로 밀어 넣는다.
//! let mut history = SessionHistory::new();
//! history.push(&PythonSource::parse("import math\nr = 2.0\n")?);
//!
//! // 2. 뒤에 한 줄을 이어 붙인 소스를 준다.
//! let grown = PythonSource::parse("import math\nr = 2.0\narea = math.pi * r ** 2\n")?;
//! let plan = history.align(&grown);
//!
//! // 앞 두 줄은 세션이 방금 실행한 바로 그것이다. 다시 돌리지 않는다.
//! let actions: Vec<Action> = plan.plans.iter().map(|p| p.action).collect();
//! assert_eq!(actions, [Action::Reuse, Action::Reuse, Action::Run]);
//!
//! // 3. Run인 것만 위에서 아래로 실행한다. 실행은 이 crate 밖의 일이다.
//! for entry in plan.run_plans() {
//!     let _source_text = std::str::from_utf8(grown.slice(entry.range)).unwrap();
//! }
//!
//! // 4. 계획을 실행 완료했으면 realize로 기록한다. 이제 이 소스가 실현 열이다.
//! history.realize(&grown);
//! assert!(history.align(&grown).run_plans().next().is_none());
//!
//! // 5. 이제 가운데 줄을 고쳐서 다시 준다.
//! let edited = PythonSource::parse("import math\nr = 3.0\narea = math.pi * r ** 2\n")?;
//! let plan = history.align(&edited);
//!
//! // import는 그대로 재사용된다 — 그 뒤의 어떤 실행도 math를 건드리지 않았다.
//! // r부터는 다시 실행한다. 편집 지점 아래는 전부 낡았기 때문이다.
//! let actions: Vec<Action> = plan.plans.iter().map(|p| p.action).collect();
//! assert_eq!(actions, [Action::Reuse, Action::Run, Action::Run]);
//! assert_eq!(plan.plans[1].reason, DecisionReason::StatementChanged);
//!
//! // 6. 실행하고 realize하면 루프는 그 자리에서 수렴한다.
//! //    영구히 못 쓰게 되는 세션은 없다.
//! history.realize(&edited);
//! assert!(history.align(&edited).run_plans().next().is_none());
//! # Ok::<(), pysash::ParseError>(())
//! ```

mod range;
pub use range::Range;

mod effect;
pub use effect::Effect;

mod parse_error;
pub use parse_error::{ParseError, ParseErrorKind};

mod diagnostic;
pub use diagnostic::Diagnostic;

mod statement_facts;
pub use statement_facts::{CalleeSummary, StatementFacts};

pub mod canonical_statement;

mod statement;
pub use statement::Statement;

pub mod python_source;

mod trace;
mod def_use;
mod summaries;

/// 지금까지 성공적으로 실행된 것의 선형 기록.
///
/// Python이나 IPython REPL에 입력하듯 [`PythonSource`](python_source::PythonSource)를
/// 순서대로 밀어 넣는다. **들어오는 소스는 성공한 실행이어야 한다** — 검증하지 않는
/// 계약이다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SessionHistory {
    /// 입력된 소스가 순서대로. 각 소스가 자기 statement를 들고 있으므로 이 하나가
    /// 곧 `SessionHistory → PythonSource → statement` 트리다.
    ///
    /// statement만 떼어 이어 붙이면 "어느 소스의 몇 번째"가 사라지고, 원문을
    /// 잘라볼 대상도 함께 사라진다. `PythonSource`는 전부 `Arc` 백업이라 보관
    /// 비용이 O(1)이다.
    sources: Vec<python_source::PythonSource>,
    /// 현재 "실현된" 선형 실행 열. 마지막 align의 소스를 그대로 실행한 상태라고
    /// 세션이 믿는 구간이다.
    realized: Vec<trace::ExecRef>,
    /// 실현 열 밖으로 밀려난 실행들. 효과는 남아 있지만 더 이상 어떤 소스의
    /// 실행으로도 세지 않는다. 오염 집합의 재료다.
    residue: Vec<trace::ExecRef>,
    /// 이름 사이의 연결 — 살아 있는 이름과 별칭 클래스.
    graph: def_use::DefUseGraph,
    /// 세션에 정의된 callable들의 요약.
    summaries: summaries::SummaryTable,
    /// 지금까지 기록된 실행의 수. 실행 순번(seq) 발급기다 — realize가 실현 열을
    /// 교체해도 순번은 절대 되돌아가지 않는다.
    executions: usize,
    /// 부분 실행 등으로 세션 상태를 더는 신뢰할 수 없다.
    poisoned: bool,
}

mod alignment_plan;
pub use alignment_plan::{Action, AlignmentPlan, DecisionReason, PlanSummary, StatementPlan};

mod record;
mod align;
mod inspect;
