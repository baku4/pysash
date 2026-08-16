use crate::source::{ParseError, ParseErrorKind};
use crate::range::Range;

/// Decodes UTF-8 input and returns its offset within the original bytes.
///
/// A UTF-8 BOM is stripped; PEP 263 declarations for other encodings are rejected.
pub fn decode(bytes: &[u8]) -> Result<(&str, u32), ParseError> {
    if bytes.len() > u32::MAX as usize {
        return Err(ParseError {
            range: Range::new(0, 0),
            kind: ParseErrorKind::TooLarge { len: bytes.len() },
        });
    }

    let (body, base) = match bytes.strip_prefix(b"\xEF\xBB\xBF".as_slice()) {
        Some(rest) => (rest, 3),
        None => (bytes, 0),
    };

    let text = match str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            let offset = base + error.valid_up_to() as u32;
            return Err(ParseError {
                range: Range::new(offset, offset),
                kind: ParseErrorKind::NotUtf8 { offset },
            });
        }
    };

    if let Some(declared) = coding_cookie(text)
        && !is_utf8_name(&declared)
    {
        return Err(ParseError {
            range: Range::new(base, base),
            kind: ParseErrorKind::UnsupportedEncoding {
                declared: declared.into_boxed_str(),
            },
        });
    }

    Ok((text, base))
}

/// Finds a PEP 263 encoding declaration in the first two comment lines.
fn coding_cookie(text: &str) -> Option<String> {
    for line in text.lines().take(2) {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            continue;
        }
        let Some(after) = trimmed.split_once("coding") else {
            continue;
        };
        let rest = after.1.trim_start();
        let Some(value) = rest.strip_prefix([':', '=']) else {
            continue;
        };
        let name: String = value
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

/// Returns whether PEP 263 treats the name as UTF-8 or an accepted subset.
fn is_utf8_name(name: &str) -> bool {
    let normalized: String = name
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect();
    matches!(
        normalized.as_str(),
        "utf8" | "u8" | "utf" | "utf8sig" | "ascii" | "usascii" | "646" | "ansix341968"
    )
}
