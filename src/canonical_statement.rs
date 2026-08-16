// module-rule: allow import-alias -- name-conflict: avoid collision with core::result::Result
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
// module-rule: allow import-alias -- name-conflict: avoid collision with std::hash::Hasher
use blake3::Hasher as Blake3Hasher;

/// A statement identity derived from a conservative canonical encoding.
///
/// Equality confirms the full encoding after comparing digests, so a hash collision cannot
/// authorize reuse. See `docs/design.md` for the normalization boundary.
#[derive(Clone)]
pub struct CanonicalStatement {
    digest: [u8; 32],
    encoding: Box<[u8]>,
}

impl CanonicalStatement {
    /// Creates an identity whose digest is derived from the supplied encoding.
    pub fn from_encoding(encoding: Vec<u8>) -> Self {
        let mut hasher = Blake3Hasher::new();
        hasher.update(&encoding);
        Self {
            digest: *hasher.finalize().as_bytes(),
            encoding: encoding.into_boxed_slice(),
        }
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
