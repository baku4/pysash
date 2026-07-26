// module-rule: allow import-alias -- name-conflict: avoid collision with core::result::Result
use std::fmt::{Display, Formatter, Result as FmtResult};
use super::Range;

/// plan의 품질에 대한 주석. 에러가 아니다.
///
/// 전부 "내가 못 본 것" 또는 "내가 가정한 것"을 드러내는 축에만 둔다. `exec()` 한 줄
/// 때문에 plan 생성이 실패하면 실사용 코드에서 매번 죽으므로, 이런 것들은 `Err`가
/// 아니라 진단으로 보고하고 판정은 안전한 쪽(Run)으로 떨어뜨린다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Diagnostic {
    /// 입력 소스가 바인딩하지 않고 읽기만 하는 이름. 이 소스는 fresh run에서
    /// 재현되지 않으며, 해당 statement는 강제 Run이다.
    UnresolvedReference { name: Box<str>, range: Range },
    /// 정적으로 다룰 수 없는 구문. `exec` / `globals` / `from m import *` 등.
    UnsupportedConstruct { construct: Box<str>, range: Range },
    /// 세션이 입력 소스의 prefix를 넘어 추가로 실행됐다. 그 잉여 실행이 무엇을
    /// 훼손했는지는 오염 집합으로 상향 근사한다.
    SessionDiverged { residue_len: usize },
    /// 잉여 실행에 반사적 구문이 있다. 오염 집합이 ⊤가 되어 전면 Run이다.
    OpaqueResidue { range: Range },
    /// 호출의 효과가 알려진 범위 안에 있다는 가정(A-KnownEffects)에 기대어 내린
    /// 재사용 판정이다.
    UnknownEffectReuse { name: Box<str>, range: Range },
}

impl Diagnostic {
    /// 이 진단이 가리키는 소스 위치. 세션 전체에 대한 진단이면 `None`이다.
    pub fn range(&self) -> Option<Range> {
        match self {
            Diagnostic::UnresolvedReference { range, .. }
            | Diagnostic::UnsupportedConstruct { range, .. }
            | Diagnostic::OpaqueResidue { range }
            | Diagnostic::UnknownEffectReuse { range, .. } => Some(*range),
            Diagnostic::SessionDiverged { .. } => None,
        }
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Diagnostic::UnresolvedReference { name, .. } => {
                write!(f, "`{name}` is read but never bound by this source")
            }
            Diagnostic::UnsupportedConstruct { construct, .. } => {
                write!(f, "`{construct}` cannot be analyzed statically")
            }
            Diagnostic::SessionDiverged { residue_len } => {
                write!(
                    f,
                    "session ran {residue_len} statement(s) beyond this source"
                )
            }
            Diagnostic::OpaqueResidue { .. } => {
                write!(
                    f,
                    "reflective construct in the residue; everything must run"
                )
            }
            Diagnostic::UnknownEffectReuse { name, .. } => {
                write!(f, "reuse assumes the effects of `{name}` are bounded")
            }
        }
    }
}
