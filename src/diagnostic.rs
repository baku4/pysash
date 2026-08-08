use super::Range;

/// plan의 품질에 대한 주석. 에러가 아니다.
///
/// 내가 못 본 것과 내가 가정한 것을 드러낸다. 이게 붙어도 plan은 유효하다 —
/// 애매한 것은 이미 Run으로 떨어졌기 때문이다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Diagnostic {
    /// 소스가 읽는 이름을 소스 안에서 찾을 수 없다. 세션에 있어야 실행되는
    /// 조각이라는 뜻이고, fresh run에서는 재현되지 않는다.
    UnresolvedReference { name: Box<str>, range: Range },
    /// 정적으로 따라갈 수 없는 구문. 안전한 쪽(Run)으로 처리했다.
    UnsupportedConstruct { construct: Box<str>, range: Range },
    /// 세션이 이 소스의 prefix를 넘어 실행했다. 그 실행들이 오염 집합의
    /// 재료가 된다.
    SessionDiverged { residue_len: usize },
    /// prefix 밖 실행에 반사적 구문이 있다 — 무엇이 오염됐는지 알 수 없어
    /// 전부 Run이다.
    OpaqueResidue { range: Range },
}
