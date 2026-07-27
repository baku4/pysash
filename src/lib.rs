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

mod decision_reason;
pub use decision_reason::{Action, DecisionReason};

mod alignment_plan;
pub use alignment_plan::{AlignmentPlan, Step};

pub mod python_source;

pub mod session_history;
