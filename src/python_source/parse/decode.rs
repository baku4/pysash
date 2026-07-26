use crate::parse_error::{ParseError, ParseErrorKind};
use crate::range::Range;

/// 입력 바이트열을 `&str`로 읽고, 그 문자열이 원본의 몇 번째 바이트에서 시작하는지를
/// 함께 돌려준다.
///
/// BOM을 떼어내므로 offset이 0이 아닐 수 있다. 이 offset을 파서가 준 위치에 더해야
/// `Range`가 사용자가 준 원본 바이트열 기준이 된다.
///
/// UTF-8이 아닌 인코딩 선언(PEP 263)은 거부한다 — `Range`가 어느 바이트열 기준인지
/// 흐려지는 것보다 명시적으로 실패하는 편이 낫다.
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

/// PEP 263 인코딩 선언을 찾는다. 첫 두 줄의 주석만 본다.
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

/// PEP 263이 UTF-8과 그 부분집합으로 인정하는 별칭들.
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
