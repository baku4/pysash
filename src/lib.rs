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

mod range;
pub use range::Range;

mod parse_error;
pub use parse_error::{ParseError, ParseErrorKind};

pub mod canonical_statement;

mod statement;
pub use statement::Statement;

pub mod python_source;

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
}

mod decision_reason;
pub use decision_reason::{Action, DecisionReason};
mod alignment_plan;
pub use alignment_plan::{AlignmentPlan, StatementPlan};

mod record;
mod align;
