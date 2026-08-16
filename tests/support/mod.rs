//! Fixture loading and compact alignment-report helpers.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use pysash::SessionHistory;
use pysash::plan::{Action, AlignmentPlan, DecisionReason, SessionDiagnostic};
use pysash::source::PythonSource;

/// A named fixture retaining its original text and parsed source.
pub struct Fixture {
    pub name: String,
    pub text: String,
    pub source: PythonSource,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(path: &Path, name: String) -> Fixture {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let source = PythonSource::parse(&text)
        .unwrap_or_else(|e| panic!("{} is not parseable python: {e}", path.display()));
    Fixture { name, text, source }
}

/// Loads every corpus source in deterministic name order.
pub fn corpus() -> Vec<Fixture> {
    let dir = fixtures_dir().join("corpus");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "py"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "corpus is empty: {}", dir.display());
    paths
        .iter()
        .map(|path| {
            let name = path.file_name().expect("file").to_string_lossy().into_owned();
            load(path, name)
        })
        .collect()
}

/// `tests/fixtures/sessions/<scenario>/<file>`.
pub fn step(scenario: &str, file: &str) -> PythonSource {
    let path = fixtures_dir().join("sessions").join(scenario).join(file);
    load(&path, format!("{scenario}/{file}")).source
}

/// Formats actions as one character per statement: `.` for reuse and `X` for run.
pub fn actions(plan: &AlignmentPlan) -> String {
    plan.steps
        .iter()
        .map(|statement| match statement.action {
            Action::Reuse => '.',
            Action::Run => 'X',
        })
        .collect()
}

pub fn reasons(plan: &AlignmentPlan) -> Vec<DecisionReason> {
    plan.steps
        .iter()
        .map(|statement| statement.reason.clone())
        .collect()
}

/// Formats each decision beside the statement's first source line.
pub fn explain(plan: &AlignmentPlan, code: &PythonSource) -> String {
    let mut out = String::new();
    for statement in &plan.steps {
        let text = String::from_utf8_lossy(code.slice(statement.range));
        let first_line = text.lines().next().unwrap_or("").trim_end();
        out.push_str(&format!(
            "{:>3} {} {:<28} {first_line}\n",
            statement.index,
            match statement.action {
                Action::Reuse => '.',
                Action::Run => 'X',
            },
            format!("{:?}", statement.reason),
        ));
    }
    out
}

pub fn has_diagnostic(plan: &AlignmentPlan, matches: impl Fn(&SessionDiagnostic) -> bool) -> bool {
    plan.diagnostics.iter().any(matches)
}

/// Returns source containing the first `count` statements and their original bytes.
pub fn head(source: &PythonSource, count: usize) -> PythonSource {
    assert!(count > 0 && count <= source.statements().len());
    let end = source.statements()[count - 1].range.end as usize;
    PythonSource::parse_bytes(&source.raw()[..end]).expect("source cut at a statement boundary")
}

/// Returns source containing only the statement at `index`.
pub fn nth(source: &PythonSource, index: usize) -> PythonSource {
    let range = source.statements()[index].range;
    PythonSource::parse_bytes(source.slice(range)).expect("one statement reparses unchanged")
}

/// Inserts one non-conflicting top-level statement at `index`.
pub fn insert_probe(source: &PythonSource, index: usize) -> PythonSource {
    let raw = source.raw();
    let at = source.statements()[index].range.start as usize;
    assert!(
        at == 0 || raw[at - 1] == b'\n',
        "statement {index} does not begin at the start of a line"
    );
    let mut bytes = Vec::with_capacity(raw.len() + PROBE.len());
    bytes.extend_from_slice(&raw[..at]);
    bytes.extend_from_slice(PROBE.as_bytes());
    bytes.extend_from_slice(&raw[at..]);
    PythonSource::parse_bytes(&bytes).expect("inserting one line preserves valid syntax")
}

/// Replaces the statement at `index` with one line without shifting later statements.
pub fn replace_statement(source: &PythonSource, index: usize) -> PythonSource {
    let raw = source.raw();
    let statement = &source.statements()[index];
    let (start, end) = (statement.range.start as usize, statement.range.end as usize);
    assert!(
        start == 0 || raw[start - 1] == b'\n',
        "statement {index} does not begin at the start of a line"
    );
    let mut bytes = Vec::with_capacity(raw.len() + PROBE.len());
    bytes.extend_from_slice(&raw[..start]);
    // Preserve the original newline and any following comments.
    bytes.extend_from_slice(PROBE.trim_end().as_bytes());
    bytes.extend_from_slice(&raw[end..]);
    PythonSource::parse_bytes(&bytes).expect("replacing one line preserves valid syntax")
}

/// Maps each statement to its jupytext `# %%` cell, or cell zero without markers.
pub fn cell_of_statement(source: &PythonSource) -> Vec<usize> {
    let raw = source.raw();
    let mut marks: Vec<usize> = Vec::new();
    let mut offset = 0;
    for line in raw.split_inclusive(|byte| *byte == b'\n') {
        if line.starts_with(b"# %%") {
            marks.push(offset);
        }
        offset += line.len();
    }
    source
        .statements()
        .iter()
        .map(|statement| {
            let at = statement.range.start as usize;
            marks.iter().rposition(|mark| *mark <= at).unwrap_or(0)
        })
        .collect()
}

/// Returns reusable and total executable cells for the plan's source cell mapping.
pub fn cell_reuse(plan: &AlignmentPlan, cells: &[usize]) -> (usize, usize) {
    let mut run: Vec<usize> = plan
        .steps
        .iter()
        .filter(|statement| statement.action == Action::Run)
        .map(|statement| cells[statement.index])
        .collect();
    run.sort_unstable();
    run.dedup();

    let mut all = cells.to_vec();
    all.sort_unstable();
    all.dedup();
    (all.len() - run.len(), all.len())
}

/// Appends one non-conflicting statement.
pub fn append_probe(source: &PythonSource) -> PythonSource {
    let mut bytes = source.raw().to_vec();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(PROBE.as_bytes());
    PythonSource::parse_bytes(&bytes).expect("appending one line preserves valid syntax")
}

/// A probe name absent from the corpus so it cannot extend a prefix accidentally.
const PROBE: &str = "pysash_probe_marker = 1\n";

/// Creates a session that has executed only this source.
pub fn realized(source: &PythonSource) -> SessionHistory {
    let mut history = SessionHistory::new();
    history.realize(source);
    history
}
