use super::canonical_statement::CanonicalStatement;
use super::{Range, StatementFacts};

/// 소스에서 떼어낸 statement 하나.
///
/// `range`는 원본 바이트열의 어디부터 어디까지인지, `canonical`은 실제로 어떤
/// statement인지를 말한다. `facts`는 의존성 판정에 쓰는 정적 사실이다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Statement {
    pub range: Range,
    pub canonical: CanonicalStatement,
    pub facts: StatementFacts,
}
