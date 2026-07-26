// module-rule: allow import-alias -- name-conflict: avoid collision with core::result::Result
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
// module-rule: allow import-alias -- name-conflict: avoid collision with std::hash::Hasher
use blake3::Hasher as Blake3Hasher;

/// statement의 정체성. 같은 코드는 같은 값이 된다.
///
/// 아래 셋은 전부 같은 `CanonicalStatement`다 — 공백도 주석도 리터럴 표기도
/// statement의 정체성이 아니다.
///
/// ```python
/// x = 1000
/// x=1000
/// x = 1_000  # comment
/// ```
///
/// 반대로 `1000`과 `1000.0`은 다르다. 과다 정규화는 잘못된 재사용이 되어 조용히
/// 틀린 결과를 낳고, 과소 정규화는 불필요한 재실행이 되어 그냥 느릴 뿐이다.
/// 애매하면 언제나 "다르다"로 떨어진다.
///
/// `digest`는 O(1) 사전 비교와 해시 키를 위한 것이고, 동일성의 최종 판정은 `encoding`
/// 바이트 비교로 확정한다 — 재사용 여부가 해시 충돌 가능성에 걸리지 않게 하기 위해서다.
#[derive(Clone)]
pub struct CanonicalStatement {
    digest: [u8; 32],
    encoding: Box<[u8]>,
}

impl CanonicalStatement {
    /// 유일한 생성 경로. `digest`는 `encoding`에서만 파생되므로 둘이 어긋난 값은
    /// 만들 수 없다.
    pub fn from_encoding(encoding: Vec<u8>) -> Self {
        let mut hasher = Blake3Hasher::new();
        hasher.update(&encoding);
        Self {
            digest: *hasher.finalize().as_bytes(),
            encoding: encoding.into_boxed_slice(),
        }
    }

    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    pub fn encoding(&self) -> &[u8] {
        &self.encoding
    }
}

impl PartialEq for CanonicalStatement {
    fn eq(&self, other: &Self) -> bool {
        self.digest == other.digest && self.encoding == other.encoding
    }
}

impl Eq for CanonicalStatement {}

impl Hash for CanonicalStatement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.digest.hash(state);
    }
}

impl Debug for CanonicalStatement {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "CanonicalStatement(")?;
        for byte in &self.digest[..6] {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}
