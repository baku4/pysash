use ruff_python_ast::{Stmt, comparable::ComparableStmt};

/// statement의 정체성을 바이트열로 적는다. 이 바이트열이 같으면 같은 statement다.
///
/// 정규화는 ruff의 `ComparableStmt`가 주는 만큼만 한다 — 공백, 개행, 주석, 괄호,
/// 리터럴 표기(`1_000` / `0x3E8`), 따옴표 종류, 식별자 NFKC 정규화까지다. 여기서
/// 한 걸음도 더 나가지 않는다. 상수 폴딩(`2*500`)이나 alpha-equivalence
/// (`def f(a)` / `def f(b)`)는 하지 않는다.
///
/// `ComparableStmt`가 모르는 것 하나를 앞에 덧붙인다 — bare string literal은 부모의
/// 첫 statement일 때만 `__doc__`이 되는데, `ComparableStmt`는 자기 위치를 모른다.
///
/// 스키마 태그도 함께 섞는다. ruff를 올려 `ComparableStmt`의 표현이 달라지면 옛
/// encoding과 자동으로 달라지므로, 낡은 판정이 조용히 살아남지 않는다.
pub fn encode(stmt: &Stmt, is_docstring: bool) -> Vec<u8> {
    let doc_tag = if is_docstring { "doc" } else { "stmt" };
    let comparable = ComparableStmt::from(stmt);
    format!("pysash-canon/1|ruff/0.0.6|{doc_tag}|{comparable:?}").into_bytes()
}
