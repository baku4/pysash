use std::sync::Arc;
use ruff_python_ast::{Expr, PySourceType, Stmt};
use ruff_python_parser::parse_unchecked_source;
use ruff_text_size::Ranged;
use crate::canonical_statement::CanonicalStatement;
use crate::parse_error::{ParseError, ParseErrorKind};
use crate::range::Range;
use crate::statement::Statement;
use super::PythonSource;
use super::canonicalize::encode;
use super::decode::decode;

impl PythonSource {
    /// UTF-8 문자열을 Python 소스로 읽는다.
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        Self::parse_bytes(source.as_bytes())
    }

    /// 바이트열을 Python 소스로 읽는다. 문법적으로 파싱 가능하지 않으면 실패한다.
    pub fn parse_bytes(source: &[u8]) -> Result<Self, ParseError> {
        let (text, base) = decode(source)?;

        let parsed = parse_unchecked_source(text, PySourceType::Python);
        if let Some(error) = parsed.errors().first() {
            return Err(ParseError {
                range: shift(
                    error.location.start().into(),
                    error.location.end().into(),
                    base,
                ),
                kind: ParseErrorKind::Syntax {
                    message: error.error.to_string().into_boxed_str(),
                },
            });
        }

        let body = &parsed.syntax().body;
        let mut statements = Vec::with_capacity(body.len());
        for (index, stmt) in body.iter().enumerate() {
            let encoding = encode(stmt, index == 0 && is_bare_string(stmt));
            statements.push(Statement {
                range: shift(stmt.start().into(), stmt.end().into(), base),
                canonical: CanonicalStatement::from_encoding(encoding),
            });
        }

        Ok(PythonSource {
            raw: Arc::from(source),
            statements: Arc::from(statements),
        })
    }
}

/// 파서가 준 위치를 원본 바이트열 기준으로 옮긴다. BOM을 떼어낸 만큼이 `base`다.
fn shift(start: u32, end: u32, base: u32) -> Range {
    Range::new(base + start, base + end)
}

/// 값이 문자열 리터럴 하나뿐인 expression statement인가. 첫 statement일 때만
/// docstring이 되므로 위치 판정은 호출부가 한다.
fn is_bare_string(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr) => matches!(*expr.value, Expr::StringLiteral(_)),
        _ => false,
    }
}
