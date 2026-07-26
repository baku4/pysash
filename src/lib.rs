//! PySASH — Python Source Alignment with Session History.
//!
//! `SessionHistory`와 새로운 Python 소스를 비교해, 각 statement의 기존 실행을
//! 재사용할 수 있는지 판단한다. 정적 분석만 수행하며 Python 코드를 실행하지 않는다.
//!
//! 재사용의 근거는 "값을 다시 계산해도 같다"가 아니라 **"그 실행이 이미 이 소스의
//! 실행이었다"**이다. `SessionHistory`는 linear하므로, 세션의 앞부분이 입력 소스의
//! 앞부분과 canonical하게 동일하면 그 실행들은 같은 프로그램을 같은 순서로 같은
//! 시작 상태에서 실행한 것이다.
//!
//! false reuse는 조용히 틀린 결과이고 false run은 낭비일 뿐이다. 이 비대칭이
//! 이 crate 전반의 판정 기준이다 — 애매하면 언제나 Run이다.

mod range;
pub use range::Range;

mod effect;
pub use effect::Effect;

mod source_mode;
pub use source_mode::SourceMode;

mod parse_error;
pub use parse_error::{ParseError, ParseErrorKind};

mod diagnostic;
pub use diagnostic::Diagnostic;

mod decision_reason;
pub use decision_reason::{Action, DecisionReason};

mod statement_facts;
pub use statement_facts::{CalleeSummary, StatementFacts};
