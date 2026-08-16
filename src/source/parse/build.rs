use std::sync::Arc;
use ruff_python_ast::{Expr, PySourceType, Stmt};
use ruff_python_parser::parse_unchecked_source;
use ruff_text_size::Ranged;
use crate::canonical_statement::CanonicalStatement;
use crate::source::{ParseError, ParseErrorKind};
use crate::range::Range;
use crate::statement::Statement;
use super::PythonSource;
use super::canonicalize::encode;
use super::decode::decode;
use super::facts::extract;

impl PythonSource {
    /// Parses a UTF-8 string as Python source.
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        Self::parse_bytes(source.as_bytes())
    }

    /// Parses bytes as UTF-8 Python source and rejects invalid syntax or encoding.
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
                facts: extract(stmt),
            });
        }

        Ok(PythonSource {
            raw: Arc::from(source),
            statements: Arc::from(statements),
        })
    }
}

/// Shifts a parser range by the stripped BOM length.
fn shift(start: u32, end: u32, base: u32) -> Range {
    Range::new(base + start, base + end)
}

/// Returns whether a statement is a bare string expression.
fn is_bare_string(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr) => matches!(*expr.value, Expr::StringLiteral(_)),
        _ => false,
    }
}
