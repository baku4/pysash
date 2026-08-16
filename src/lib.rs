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
//! 핵심 domain은 둘이다: 실행 기록인 [`SessionHistory`]와 입력인
//! [`PythonSource`](source::PythonSource). 나머지는 이 둘의 입출력 어휘다 —
//! [`source`]에 입력 쪽이, [`plan`]에 판정 결과 쪽이 산다.
//!
//! # 쓰는 법
//!
//! ```
//! use pysash::SessionHistory;
//! use pysash::plan::{Action, DecisionReason};
//! use pysash::source::PythonSource;
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
//! let actions: Vec<Action> = plan.steps.iter().map(|p| p.action).collect();
//! assert_eq!(actions, [Action::Reuse, Action::Reuse, Action::Run]);
//!
//! // 3. Run인 것만 위에서 아래로 실행한다. 실행은 이 crate 밖의 일이다.
//! for entry in plan.run_steps() {
//!     let _source_text = std::str::from_utf8(grown.slice(entry.range)).unwrap();
//! }
//!
//! // 4. 계획을 실행 완료했으면 realize로 기록한다. 이제 이 소스가 실현 열이다.
//! history.realize(&grown);
//! assert!(history.align(&grown).run_steps().next().is_none());
//!
//! // 5. 이제 가운데 줄을 고쳐서 다시 준다.
//! let edited = PythonSource::parse("import math\nr = 3.0\narea = math.pi * r ** 2\n")?;
//! let plan = history.align(&edited);
//!
//! // import는 그대로 재사용된다 — 그 뒤의 어떤 실행도 math를 건드리지 않았다.
//! // r부터는 다시 실행한다. 편집 지점 아래는 전부 낡았기 때문이다.
//! let actions: Vec<Action> = plan.steps.iter().map(|p| p.action).collect();
//! assert_eq!(actions, [Action::Reuse, Action::Run, Action::Run]);
//! assert_eq!(plan.steps[1].reason, DecisionReason::StatementChanged);
//!
//! // 6. 실행하고 realize하면 루프는 그 자리에서 수렴한다.
//! //    영구히 못 쓰게 되는 세션은 없다.
//! history.realize(&edited);
//! assert!(history.align(&edited).run_steps().next().is_none());
//! # Ok::<(), pysash::source::ParseError>(())
//! ```

mod range;
pub use range::Range;

pub mod plan;

mod statement_facts;
mod canonical_statement;
mod statement;

pub mod source;

mod trace;
mod def_use;
mod summaries;

/// 지금까지 세션에서 일어난 실행의 선형 기록.
///
/// Python이나 IPython REPL에 입력하듯 [`PythonSource`](source::PythonSource)를
/// 순서대로 밀어 넣는다. **[`push`](Self::push)와 [`realize`](Self::realize)로
/// 들어오는 소스는 성공한 실행이어야 한다** — 검증하지 않는 계약이다. 중간에 끊겨
/// 어디까지 돌았는지 모르는 실행은 [`record_partial`](Self::record_partial)이 받는다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SessionHistory {
    /// 현재 "실현된" 선형 실행 열. 마지막 align의 소스를 그대로 실행한 상태라고
    /// 세션이 믿는 구간이다.
    ///
    /// 각 실행이 자기 소스를 직접 소유하므로 세션은 소스 목록을 따로 들지 않는다.
    /// 어떤 실행도 가리키지 않게 된 소스는 그 자리에서 해제된다.
    realized: Vec<trace::ExecRef>,
    /// 실현 열 밖의 실행들 — 밀려난 옛 실행과 중간에 끊긴 실행. 효과는 남아
    /// 있지만 더 이상 어떤 소스의 실행으로도 세지 않는다. 오염 집합의 재료다.
    ///
    /// 누계가 아니다. 어떤 판정에도 닿을 수 없게 된 것은 [`forget`]이 버리므로,
    /// 편집 루프를 아무리 돌아도 판정에 쓰이는 만큼만 남는다.
    residue: Vec<trace::ExecRef>,
    /// 이름 사이의 연결 — 살아 있는 이름과 별칭 클래스.
    graph: def_use::DefUseGraph,
    /// 세션에 정의된 callable들의 요약.
    summaries: summaries::SummaryTable,
    /// 지금까지 기록된 실행의 수. 실행 순번(seq) 발급기다 — realize가 실현 열을
    /// 교체해도 순번은 절대 되돌아가지 않는다.
    executions: usize,
    /// 세션에 무슨 일이 있었는지 알 수 없다. 켜지면 이후 모든 정렬이 전면 Run이다.
    poisoned: bool,
}

mod disturbance;
mod forget;

mod record;
mod align;
mod inspect;
