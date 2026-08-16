//! Conservative identity boundaries for `CanonicalStatement`.

use pysash::source::PythonSource;

/// Returns the inferred canonical identity of a source's first statement.
macro_rules! canon {
    ($source:expr) => {{
        let source = $source;
        let parsed = PythonSource::parse(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
        assert_eq!(
            parsed.statements().len(),
            1,
            "{source:?} should be exactly one statement"
        );
        parsed.statements()[0].canonical.clone()
    }};
}

fn assert_same(pairs: &[(&str, &str)]) {
    for (a, b) in pairs {
        assert_eq!(canon!(a), canon!(b), "expected {a:?} == {b:?}");
    }
}

fn assert_differ(pairs: &[(&str, &str)]) {
    for (a, b) in pairs {
        assert_ne!(canon!(a), canon!(b), "expected {a:?} != {b:?}");
    }
}

/// Formatting, comments, and equivalent literal spellings do not change identity.
#[test]
fn spacing_comments_and_digit_separators_are_one_statement() {
    let a = canon!("x = 1000\n");
    let b = canon!("x=1000\n");
    let c = canon!("x = 1_000  # comment\n");

    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, c);
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

/// Ruff applies the same identifier NFKC normalization as CPython.
#[test]
fn tier0_identifiers_are_nfkc_normalized() {
    assert_same(&[("\u{1D54F} = 1\n", "X = 1\n")]);
}

/// Equal-looking values of different types remain distinct.
#[test]
fn tier1_numeric_types_are_distinct() {
    assert_differ(&[
        ("x = 1000\n", "x = 1000.0\n"),
        // `1e3` is a float and remains distinct from integer `1000`.
        ("x = 1000\n", "x = 1e3\n"),
        ("x = True\n", "x = 1\n"),
        ("x = 0.0\n", "x = -0.0\n"),
    ]);
}

/// Runtime f-string differences are not normalized away.
#[test]
fn tier1_fstrings_are_distinct() {
    assert_differ(&[
        ("s = f'a'\n", "s = 'a'\n"),
        ("s = f'{a}'\n", "s = f'{a!r}'\n"),
        // Self-documenting `=` preserves surrounding whitespace in its output.
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

/// A bare string has docstring identity only in the first statement position.
#[test]
fn docstring_position_changes_identity() {
    let as_docstring = PythonSource::parse("'doc'\n").unwrap();
    let as_plain_expression = PythonSource::parse("x = 1\n'doc'\n").unwrap();

    assert_ne!(
        as_docstring.statements()[0].canonical,
        as_plain_expression.statements()[1].canonical
    );
}
