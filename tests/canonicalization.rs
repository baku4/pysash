//! `CanonicalStatement`의 동일성 경계.
//!
//! 과다 정규화는 잘못된 재사용이 되어 조용히 틀린 결과를 낳고, 과소 정규화는
//! 불필요한 재실행이 되어 그냥 느릴 뿐이다. 그래서 "같다"로 판정하는 목록보다
//! "다르다"로 판정하는 목록이 더 중요하다.

use pysash::canonical_statement::CanonicalStatement;
use pysash::python_source::PythonSource;

fn canon(source: &str) -> CanonicalStatement {
    let parsed = PythonSource::parse(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
    assert_eq!(
        parsed.statements().len(),
        1,
        "{source:?} should be exactly one statement"
    );
    parsed.statements()[0].canonical.clone()
}

fn assert_same(pairs: &[(&str, &str)]) {
    for (a, b) in pairs {
        assert_eq!(canon(a), canon(b), "expected {a:?} == {b:?}");
    }
}

fn assert_differ(pairs: &[(&str, &str)]) {
    for (a, b) in pairs {
        assert_ne!(canon(a), canon(b), "expected {a:?} != {b:?}");
    }
}

/// Architecture.md가 명시한 예시. 이 셋은 완전히 동일해야 한다.
#[test]
fn architecture_example_three_forms_are_one_statement() {
    let a = canon("x = 1000\n");
    let b = canon("x=1000\n");
    let c = canon("x = 1_000  # comment\n");

    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a.digest(), b.digest());
    assert_eq!(a.encoding(), c.encoding());
}

#[test]
fn tier0_trivia_is_normalized() {
    assert_same(&[
        ("x = 1000\n", "x=1000\n"),
        ("x = 1\r\n", "x = 1\n"),
        ("x = 1 + \\\n    2\n", "x = 1 + 2\n"),
        ("x = 1  # trailing comment\n", "x = 1\n"),
        ("x = 1\n", "x = 1"),
    ]);
}

#[test]
fn tier0_literal_spelling_is_normalized() {
    assert_same(&[
        ("x = 1000\n", "x = 0x3E8\n"),
        ("x = 1000\n", "x = 0o1750\n"),
        ("x = 1000\n", "x = 1_000\n"),
        ("s = 'a'\n", "s = \"a\"\n"),
        ("s = 'a'\n", "s = \"\"\"a\"\"\"\n"),
        ("s = 'a' 'b'\n", "s = 'ab'\n"),
        ("s = r'\\n'\n", "s = '\\\\n'\n"),
        ("b = b'\\x41'\n", "b = b'A'\n"),
    ]);
}

#[test]
fn tier0_redundant_syntax_is_normalized() {
    assert_same(&[
        ("t = 1, 2\n", "t = (1, 2)\n"),
        ("t = (1, 2)\n", "t = (1, 2,)\n"),
        ("x = ((1))\n", "x = 1\n"),
        ("if x: pass\n", "if x:\n    pass\n"),
    ]);
}

/// ruff는 식별자를 CPython과 동일하게 NFKC 정규화한다.
#[test]
fn tier0_identifiers_are_nfkc_normalized() {
    assert_same(&[("\u{1D54F} = 1\n", "X = 1\n")]);
}

/// 값이 같아 보여도 타입이 다르면 다른 statement다.
#[test]
fn tier1_numeric_types_are_distinct() {
    assert_differ(&[
        ("x = 1000\n", "x = 1000.0\n"),
        // `1e3`은 float 1000.0이다 — int `1000`과 다르다.
        ("x = 1000\n", "x = 1e3\n"),
        ("x = True\n", "x = 1\n"),
        ("x = 0.0\n", "x = -0.0\n"),
    ]);
}

/// f-string은 런타임 평가다. 겉모양이 같아도 정규화하지 않는다.
#[test]
fn tier1_fstrings_are_distinct() {
    assert_differ(&[
        ("s = f'a'\n", "s = 'a'\n"),
        ("s = f'{a}'\n", "s = f'{a!r}'\n"),
        // self-documenting `=`는 앞뒤 공백까지 그대로 출력한다.
        ("s = f'{a=}'\n", "s = f'{ a = }'\n"),
    ]);
}

#[test]
fn tier1_structure_is_never_folded() {
    assert_differ(&[
        ("x = 2*500\n", "x = 1000\n"),
        ("def f(a): pass\n", "def f(b): pass\n"),
        ("import os\n", "import os as os\n"),
        ("x = a\n", "x = b\n"),
        ("if x: pass\n", "if y: pass\n"),
    ]);
}

/// bare string은 부모의 첫 statement일 때만 `__doc__`이 된다. `ComparableStmt`는
/// 자기 위치를 모르므로 위치를 encoding에 함께 섞는다.
#[test]
fn docstring_position_changes_identity() {
    let as_docstring = PythonSource::parse("'doc'\n").unwrap();
    let as_plain_expression = PythonSource::parse("x = 1\n'doc'\n").unwrap();

    assert_ne!(
        as_docstring.statements()[0].canonical,
        as_plain_expression.statements()[1].canonical
    );
}
