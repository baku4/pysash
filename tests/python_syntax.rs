//! The Python grammar PySASH accepts.
//!
//! PySASH has no grammar of its own; every construct it can read comes from the pinned Ruff
//! parser. This file is the record of what that pin buys, so a failure here means the pin
//! changed what PySASH can read. Update this file and the README section together.
//!
//! The boundary is CPython's `ast.parse`, not `compile`. PySASH parses; it does not build a
//! symbol table, so grammatical source that no interpreter would run still yields statements.
//! Three independent axes pin that boundary, and none implies another:
//!
//! - [`ADDED`] and [`REMOVED`]: which CPython release changed a construct.
//! - [`GRAMMAR`]: which AST node kinds are reachable at all.
//! - [`PARSES_BUT_CANNOT_EXECUTE`]: grammatical source `compile` rejects.
//!
//! PySASH accepts every construct regardless of the interpreter the session runs, because the
//! two failure directions are not symmetric: rejecting syntax the session can execute makes the
//! tool unusable, while accepting syntax the session cannot execute only leaves the statement
//! unmatched, and therefore `Run`.

use pysash::source::PythonSource;
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{Expr, PySourceType, PythonVersion, Stmt};
use ruff_python_parser::{ParseOptions, parse_unchecked};
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Axis 1: the CPython release that changed each construct
// ---------------------------------------------------------------------------

/// Constructs CPython added in a given release, each with a minimal example.
///
/// The list mirrors Ruff's version gates. The version is the release that introduced the
/// syntax, not a release PySASH requires.
#[rustfmt::skip]
const ADDED: &[(&str, &str, &str)] = &[
    ("3.8", "walrus", "if (n := 1):\n    pass\n"),
    ("3.8", "positional-only parameter", "def f(a, /, b):\n    pass\n"),
    ("3.8", "star tuple in return", "def f():\n    return *a, b\n"),
    ("3.8", "star tuple in yield", "def f():\n    yield *a, b\n"),
    ("3.9", "relaxed decorator", "@buttons[0].clicked\ndef f():\n    pass\n"),
    ("3.9", "parenthesized context manager", "with (open('a') as f, open('b') as g):\n    pass\n"),
    ("3.9", "unparenthesized walrus in index", "x[a := 1]\n"),
    ("3.9", "unparenthesized walrus in set literal", "{a := 1}\n"),
    ("3.9", "unparenthesized unpacking in for", "for x in *a, *b:\n    pass\n"),
    ("3.10", "match statement", "match x:\n    case 1:\n        pass\n"),
    ("3.11", "except*", "try:\n    pass\nexcept* TypeError:\n    pass\n"),
    ("3.11", "star expression in index", "x[*a]\n"),
    ("3.11", "star annotation", "def f(*args: *Ts):\n    pass\n"),
    ("3.12", "type parameter list", "def f[T](x: T) -> T:\n    return x\n"),
    ("3.12", "type alias statement", "type Alias = int\n"),
    ("3.12", "nested quote in f-string", "y = f\"{\"nested\"}\"\n"),
    ("3.13", "type parameter default", "def f[T = int](x: T):\n    pass\n"),
    ("3.14", "template string", "y = t\"hello {name}\"\n"),
    ("3.14", "unparenthesized exception types", "try:\n    pass\nexcept A, B:\n    pass\n"),
    ("3.15", "lazy import", "lazy import os\n"),
    ("3.15", "unpacking in list comprehension", "[*x for x in y]\n"),
    ("3.15", "unpacking in dict comprehension", "{**d for d in ds}\n"),
];

/// The newest CPython release whose syntax PySASH is known to read.
///
/// Raising this is a deliberate change: it widens the input the alignment model must stay
/// correct for.
const NEWEST: &str = "3.15";

/// Constructs CPython removed in a given release, each with a minimal example.
///
/// PySASH still parses these. They cannot execute on any supported interpreter, so the
/// statement never enters a history and alignment always reaches `Run`.
#[rustfmt::skip]
const REMOVED: &[(&str, &str, &str)] = &[
    ("3.8", "parenthesized keyword argument name", "f((a)=1)\n"),
];

// ---------------------------------------------------------------------------
// Axis 2: grammatical source no interpreter will run
// ---------------------------------------------------------------------------

/// Source CPython parses but refuses to compile, each labelled with the reason `compile` gives.
///
/// PySASH stops where `ast.parse` stops, so these become statements. The session then fails to
/// execute them, no execution is recorded, and alignment reaches `Run` for anything after.
#[rustfmt::skip]
const PARSES_BUT_CANNOT_EXECUTE: &[(&str, &str)] = &[
    ("'return' outside function", "return 1\n"),
    ("'yield' outside function", "yield 1\n"),
    ("nonlocal declaration not allowed at module level", "nonlocal x\n"),
    ("'break' outside loop", "break\n"),
    ("'continue' not properly in loop", "continue\n"),
    ("'await' outside async function", "def f():\n    await g()\n"),
    ("'await' outside function", "y = f'{await g()}'\n"),
    ("duplicate argument 'a' in function definition", "def f(a, a):\n    pass\n"),
    ("no binding for nonlocal 'y' found", "def f():\n    nonlocal y\n"),
    ("name 'x' is assigned to before global declaration", "def f():\n    x = 1\n    global x\n"),
    ("can't use starred expression here", "*a\n"),
];

// ---------------------------------------------------------------------------
// Axis 3: the AST node kinds reachable from accepted source
// ---------------------------------------------------------------------------

/// One example per node kind the parser can produce, each a single top-level statement.
///
/// Together these must reach every kind in [`STMT_KINDS`] and [`EXPR_KINDS`], which is what
/// makes this a coverage measure rather than a sample.
#[rustfmt::skip]
const GRAMMAR: &[&str] = &[
    // Statements
    "def f():\n    pass\n",
    "async def f():\n    await g()\n",
    "class C(B, metaclass=M):\n    pass\n",
    "def f():\n    return 1\n",
    "del x\n",
    "type Alias = int\n",
    "x = y = 1\n",
    "x += 1\n",
    "x: int = 1\n",
    "for i in r:\n    pass\nelse:\n    pass\n",
    "async def f():\n    async for i in r:\n        pass\n",
    "while c:\n    break\nelse:\n    pass\n",
    "if c:\n    pass\nelif d:\n    pass\nelse:\n    pass\n",
    "with a as b, c:\n    pass\n",
    "async def f():\n    async with a as b:\n        pass\n",
    "match x:\n    case [1, *rest] if rest:\n        pass\n    case {'k': v, **extra}:\n        pass\n    case C(a, b=1) | None:\n        pass\n    case _:\n        pass\n",
    "raise E('m') from cause\n",
    "try:\n    pass\nexcept E as e:\n    pass\nelse:\n    pass\nfinally:\n    pass\n",
    "assert c, 'message'\n",
    "import os.path as p, sys\n",
    "from . import a as b\n",
    "def f():\n    global g\n",
    "def outer():\n    def inner():\n        nonlocal n\n",
    "while c:\n    continue\n",
    // Expressions
    "a and b or not c\n",
    "(a := 1)\n",
    "a + b * c // d % e ** f @ g\n",
    "-a\n",
    "lambda x, *args, k=1, **kw: x\n",
    "a if c else b\n",
    "{1: 2, **rest}\n",
    "{1, 2}\n",
    "[x for x in y if x]\n",
    "{x for x in y}\n",
    "{k: v for k, v in y}\n",
    "(x for x in y)\n",
    "def f():\n    yield 1\n",
    "def f():\n    yield from g\n",
    "a < b <= c\n",
    "f(1, *rest, k=2, **kw)\n",
    "y = f'{a!r:>{width}} literal'\n",
    "y = t'{a}'\n",
    "y = 'implicit' 'concatenation'\n",
    "y = b'bytes'\n",
    "y = 1 + 1.5 + 2j\n",
    "y = True\n",
    "y = None\n",
    "y = ...\n",
    "a.b.c\n",
    "a[b]\n",
    "a[1:2:3, ::-1]\n",
    "[1, 2]\n",
    "(1, 2)\n",
];

/// Every `Stmt` kind the parser can produce from Python source.
///
/// `IpyEscapeCommand` is absent by design: PySASH parses Python, not IPython, so it is
/// unreachable and [`ipython_escapes_are_rejected`] pins that.
#[rustfmt::skip]
const STMT_KINDS: &[&str] = &[
    "AnnAssign", "Assert", "Assign", "AugAssign", "Break", "ClassDef", "Continue", "Delete",
    "Expr", "For", "FunctionDef", "Global", "If", "Import", "ImportFrom", "Match", "Nonlocal",
    "Pass", "Raise", "Return", "Try", "TypeAlias", "While", "With",
];

/// Every `Expr` kind the parser can produce from Python source.
#[rustfmt::skip]
const EXPR_KINDS: &[&str] = &[
    "Attribute", "Await", "BinOp", "BoolOp", "BooleanLiteral", "BytesLiteral", "Call", "Compare",
    "Dict", "DictComp", "EllipsisLiteral", "FString", "Generator", "If", "Lambda", "List",
    "ListComp", "Name", "Named", "NoneLiteral", "NumberLiteral", "Set", "SetComp", "Slice",
    "Starred", "StringLiteral", "Subscript", "TString", "Tuple", "UnaryOp", "Yield", "YieldFrom",
];

// ---------------------------------------------------------------------------
// Axis 1 tests
// ---------------------------------------------------------------------------

#[test]
fn every_construct_added_by_a_python_release_parses() {
    for (version, construct, source) in ADDED {
        let parsed = PythonSource::parse(source)
            .unwrap_or_else(|e| panic!("Python {version} {construct} should parse: {e}"));
        assert_eq!(
            parsed.statements().len(),
            1,
            "Python {version} {construct} should be one statement"
        );
    }
}

#[test]
fn every_construct_removed_by_a_python_release_still_parses() {
    for (version, construct, source) in REMOVED {
        assert!(
            PythonSource::parse(source).is_ok(),
            "{construct}, removed in Python {version}, should still parse"
        );
    }
}

#[test]
fn the_recorded_table_reaches_the_recorded_release() {
    let newest = ADDED
        .iter()
        .map(|(version, _, _)| *version)
        .max_by_key(|version| minor(version))
        .expect("the table is not empty");

    assert_eq!(
        newest, NEWEST,
        "the table reaches Python {newest}; update NEWEST and the README section"
    );
}

/// The tables record what PySASH is known to read; they cannot notice the parser learning more.
/// Asking Ruff directly closes that gap, because PySASH does not gate on a target version and so
/// inherits the parser's newest grammar.
#[test]
fn the_pinned_parser_reaches_no_further_than_the_recorded_release() {
    assert_eq!(
        PythonVersion::latest_preview().to_string(),
        NEWEST,
        "the pinned parser moved; record the new constructs and update the README section"
    );
}

/// The target version is inert: it selects which version diagnostics Ruff reports, and PySASH
/// reads none of them. Nothing about acceptance or the resulting tree depends on it.
#[test]
fn the_target_version_changes_neither_acceptance_nor_the_tree() {
    for (_, construct, source) in ADDED {
        let mut trees = BTreeSet::new();
        for version in PythonVersion::iter() {
            let options = ParseOptions::from(PySourceType::Python).with_target_version(version);
            let parsed = parse_unchecked(source, options);
            assert!(
                parsed.errors().is_empty(),
                "{construct} should parse on target {version}"
            );
            trees.insert(format!("{:?}", parsed.syntax()));
        }
        assert_eq!(
            trees.len(),
            1,
            "{construct} should parse to one tree on every target"
        );
    }
}

// ---------------------------------------------------------------------------
// Axis 2 tests
// ---------------------------------------------------------------------------

#[test]
fn grammatical_source_parses_even_when_no_interpreter_would_run_it() {
    for (reason, source) in PARSES_BUT_CANNOT_EXECUTE {
        let parsed = PythonSource::parse(source)
            .unwrap_or_else(|e| panic!("{source:?} ({reason}) should still parse: {e}"));
        assert_eq!(
            parsed.statements().len(),
            1,
            "{source:?} should be one statement"
        );
    }
}

// ---------------------------------------------------------------------------
// Axis 3 tests
// ---------------------------------------------------------------------------

#[test]
fn every_grammar_example_is_one_statement() {
    for source in GRAMMAR {
        let parsed =
            PythonSource::parse(source).unwrap_or_else(|e| panic!("{source:?} should parse: {e}"));
        assert_eq!(
            parsed.statements().len(),
            1,
            "{source:?} should be one statement"
        );
    }
}

#[test]
fn the_grammar_examples_reach_every_node_kind() {
    let mut seen = KindCollector::default();
    for source in GRAMMAR {
        let parsed = parse_unchecked(source, ParseOptions::from(PySourceType::Python));
        for stmt in &parsed.syntax().as_module().expect("module").body {
            seen.visit_stmt(stmt);
        }
    }

    let missing_stmts: Vec<_> = STMT_KINDS
        .iter()
        .filter(|k| !seen.stmts.contains(**k))
        .collect();
    assert!(
        missing_stmts.is_empty(),
        "no example reaches Stmt::{missing_stmts:?}"
    );

    let missing_exprs: Vec<_> = EXPR_KINDS
        .iter()
        .filter(|k| !seen.exprs.contains(**k))
        .collect();
    assert!(
        missing_exprs.is_empty(),
        "no example reaches Expr::{missing_exprs:?}"
    );
}

#[test]
fn ipython_escapes_are_rejected() {
    for source in ["%time f()\n", "!ls\n", "?obj\n", "x = %magic\n"] {
        assert!(
            PythonSource::parse(source).is_err(),
            "{source:?} is IPython, not Python, and should be rejected"
        );
    }
}

/// Every entry here is also a `SyntaxError` from CPython's `ast.parse`, which is the boundary
/// PySASH matches.
#[test]
fn ungrammatical_source_is_rejected() {
    for source in [
        "def (\n",
        "def f(:\n    pass\n",
        "x = = 1\n",
        "x = (1\n",
        "class:\n",
        "if True\n    pass\n",
        "1 = x\n",
    ] {
        assert!(
            PythonSource::parse(source).is_err(),
            "{source:?} should be rejected"
        );
    }
}

// ---------------------------------------------------------------------------
// Support
// ---------------------------------------------------------------------------

/// Records which node kinds a tree contains.
#[derive(Default)]
struct KindCollector {
    stmts: BTreeSet<&'static str>,
    exprs: BTreeSet<&'static str>,
}

impl<'a> Visitor<'a> for KindCollector {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.stmts.insert(stmt_kind(stmt));
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.exprs.insert(expr_kind(expr));
        walk_expr(self, expr);
    }
}

/// Names a statement kind.
///
/// The match is exhaustive on purpose: a new Ruff node kind breaks the build here rather than
/// widening the accepted grammar unnoticed.
fn stmt_kind(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::FunctionDef(_) => "FunctionDef",
        Stmt::ClassDef(_) => "ClassDef",
        Stmt::Return(_) => "Return",
        Stmt::Delete(_) => "Delete",
        Stmt::TypeAlias(_) => "TypeAlias",
        Stmt::Assign(_) => "Assign",
        Stmt::AugAssign(_) => "AugAssign",
        Stmt::AnnAssign(_) => "AnnAssign",
        Stmt::For(_) => "For",
        Stmt::While(_) => "While",
        Stmt::If(_) => "If",
        Stmt::With(_) => "With",
        Stmt::Match(_) => "Match",
        Stmt::Raise(_) => "Raise",
        Stmt::Try(_) => "Try",
        Stmt::Assert(_) => "Assert",
        Stmt::Import(_) => "Import",
        Stmt::ImportFrom(_) => "ImportFrom",
        Stmt::Global(_) => "Global",
        Stmt::Nonlocal(_) => "Nonlocal",
        Stmt::Expr(_) => "Expr",
        Stmt::Pass(_) => "Pass",
        Stmt::Break(_) => "Break",
        Stmt::Continue(_) => "Continue",
        Stmt::IpyEscapeCommand(_) => "IpyEscapeCommand",
    }
}

/// Names an expression kind, exhaustively for the same reason as [`stmt_kind`].
fn expr_kind(expr: &Expr) -> &'static str {
    match expr {
        Expr::BoolOp(_) => "BoolOp",
        Expr::Named(_) => "Named",
        Expr::BinOp(_) => "BinOp",
        Expr::UnaryOp(_) => "UnaryOp",
        Expr::Lambda(_) => "Lambda",
        Expr::If(_) => "If",
        Expr::Dict(_) => "Dict",
        Expr::Set(_) => "Set",
        Expr::ListComp(_) => "ListComp",
        Expr::SetComp(_) => "SetComp",
        Expr::DictComp(_) => "DictComp",
        Expr::Generator(_) => "Generator",
        Expr::Await(_) => "Await",
        Expr::Yield(_) => "Yield",
        Expr::YieldFrom(_) => "YieldFrom",
        Expr::Compare(_) => "Compare",
        Expr::Call(_) => "Call",
        Expr::FString(_) => "FString",
        Expr::TString(_) => "TString",
        Expr::StringLiteral(_) => "StringLiteral",
        Expr::BytesLiteral(_) => "BytesLiteral",
        Expr::NumberLiteral(_) => "NumberLiteral",
        Expr::BooleanLiteral(_) => "BooleanLiteral",
        Expr::NoneLiteral(_) => "NoneLiteral",
        Expr::EllipsisLiteral(_) => "EllipsisLiteral",
        Expr::Attribute(_) => "Attribute",
        Expr::Subscript(_) => "Subscript",
        Expr::Starred(_) => "Starred",
        Expr::Name(_) => "Name",
        Expr::List(_) => "List",
        Expr::Tuple(_) => "Tuple",
        Expr::Slice(_) => "Slice",
        Expr::IpyEscapeCommand(_) => "IpyEscapeCommand",
    }
}

/// Returns the minor component of a `3.N` version string.
fn minor(version: &str) -> u32 {
    version
        .split_once('.')
        .and_then(|(_, minor)| minor.parse().ok())
        .unwrap_or_else(|| panic!("{version:?} is not a `3.N` version"))
}
