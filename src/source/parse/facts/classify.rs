use ruff_python_ast::Stmt;
use crate::plan::Effect;
use super::scan::MentionScan;

const OUTPUT_CALLS: &[&str] = &["display", "pprint", "print"];
const EXTERNAL_READ_CALLS: &[&str] = &["input", "open"];
const EXTERNAL_READ_METHODS: &[&str] = &[
    "load", "loads", "read", "read_bytes", "read_text", "readline", "readlines", "urlopen",
];
const EXTERNAL_WRITE_METHODS: &[&str] = &[
    "dump", "dumps", "save", "savefig", "to_csv", "to_excel", "to_json", "to_parquet",
    "to_pickle", "to_sql", "write", "write_bytes", "write_text", "writelines",
];
const NONDET_ROOTS: &[&str] = &["datetime", "random", "secrets", "time", "uuid"];

/// Classifies statement effects for caller policy without affecting reuse safety.
pub fn classify(stmt: &Stmt, scan: &MentionScan, opaque: bool) -> Effect {
    if opaque {
        return Effect::Opaque;
    }
    if matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)) {
        return Effect::Import;
    }
    let method_in = |set: &[&str]| scan.method_calls.iter().any(|m| set.contains(&m.as_str()));
    let bare_in = |set: &[&str]| scan.bare_calls.iter().any(|c| set.contains(&c.as_str()));
    if method_in(EXTERNAL_WRITE_METHODS) || scan.mentions.iter().any(|name| name == "subprocess") {
        return Effect::ExternalWrite;
    }
    if bare_in(EXTERNAL_READ_CALLS)
        || method_in(EXTERNAL_READ_METHODS)
        || scan.method_calls.iter().any(|m| m.starts_with("read_"))
    {
        return Effect::ExternalRead;
    }
    if scan
        .mentions
        .iter()
        .any(|name| NONDET_ROOTS.contains(&name.as_str()))
    {
        return Effect::Nondeterministic;
    }
    if bare_in(OUTPUT_CALLS) {
        return Effect::Output;
    }
    Effect::Pure
}
