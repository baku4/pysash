use ruff_python_ast::{Stmt, comparable::ComparableStmt};

/// Encodes conservative statement identity with docstring and schema tags.
///
/// Equal encodings represent the same statement under the pinned Ruff version.
pub fn encode(stmt: &Stmt, is_docstring: bool) -> Vec<u8> {
    let doc_tag = if is_docstring { "doc" } else { "stmt" };
    let comparable = ComparableStmt::from(stmt);
    format!("pysash-canon/1|ruff/0.0.6|{doc_tag}|{comparable:?}").into_bytes()
}
