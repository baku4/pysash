use super::canonical_statement::CanonicalStatement;
use super::Range;

/// 소스에서 떼어낸 statement 하나.
///
/// `range`는 원본 바이트열의 어디부터 어디까지인지, `canonical`은 실제로 어떤
/// statement인지를 말한다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Statement {
    pub range: Range,
    pub canonical: CanonicalStatement,
}
