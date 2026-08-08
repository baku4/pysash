use ruff_python_ast::Stmt;
use crate::statement_facts::StatementFacts;

/// statement 하나에서 [`StatementFacts`]를 뽑아낸다.
///
/// 모든 집합은 상향 근사다. 스코프를 완전히 모델링하지 못하는 구성을 만나면 그
/// statement는 opaque로 떨어진다 — whitelist 방향이다.
pub fn extract(stmt: &Stmt) -> StatementFacts {
    let scan = scan::MentionScan::run(stmt);
    let sink = walk::ExecWalker::run(stmt);

    let mut mutates = sink.mutates;
    let mut calls = sink.calls;
    let mut opaque = scan.opaque || sink.opaque;
    // 이름에 묶이지 않은 callable(인자로 넘긴 lambda 등)은 언제 불릴지 모른다.
    // 그 효과를 이 statement의 것으로 흡수한다.
    for loose in &sink.loose {
        for name in &loose.mutates_frees {
            push_unique(&mut mutates, name);
        }
        for name in &loose.global_writes {
            push_unique(&mut mutates, name);
        }
        for name in &loose.callees {
            push_unique(&mut calls, name);
        }
        opaque |= loose.opaque;
    }

    let effect = classify::classify(stmt, &scan, opaque);

    let mut mentions = scan.mentions;
    for name in sink.binds.iter().chain(&sink.deletes).chain(&calls) {
        push_unique(&mut mentions, name);
    }

    let summary = sink.nested.into_iter().reduce(|mut merged, next| {
        merged.absorb(&next);
        merged
    });

    StatementFacts {
        binds: box_names(sink.binds),
        reads: box_names(sink.reads),
        mentions: box_names(mentions),
        alias_edges: sink
            .alias_edges
            .into_iter()
            .map(|(a, b)| (a.into_boxed_str(), b.into_boxed_str()))
            .collect(),
        calls: box_names(calls),
        summary,
        deletes: box_names(sink.deletes),
        mutates: box_names(mutates),
        effect,
        opaque,
    }
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|existing| existing == name) {
        names.push(name.to_string());
    }
}

fn box_names(names: Vec<String>) -> Vec<Box<str>> {
    names.into_iter().map(String::into_boxed_str).collect()
}

mod scan;
mod walk;

mod classify;
