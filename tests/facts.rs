//! 사실 추출(M2)의 완료 판정 — Design.md의 실측 누출 케이스들.

use pysash::plan::Effect;
use pysash::source::PythonSource;

/// 소스 하나의 첫 statement가 가진 facts. 타입은 crate 밖에서 이름 붙일 수
/// 없으므로 (내부 어휘다) 추론에 맡긴다.
macro_rules! facts {
    ($code:expr $(,)?) => {{
        let source = PythonSource::parse($code).expect("valid python");
        assert_eq!(source.statements().len(), 1, "one statement expected");
        source.statements()[0].facts.clone()
    }};
}

fn names(list: &[Box<str>]) -> Vec<&str> {
    list.iter().map(|name| &**name).collect()
}

#[test]
fn function_parameters_do_not_leak() {
    let facts = facts!("def f(a):\n    return a + y\n");
    assert_eq!(names(&facts.binds), ["f"]);
    assert_eq!(names(&facts.reads), ["y"]);
}

#[test]
fn comprehension_variables_do_not_leak() {
    let facts = facts!("[q for q in y]\n");
    assert!(facts.binds.is_empty());
    assert_eq!(names(&facts.reads), ["y"]);
}

#[test]
fn lambda_parameters_do_not_leak() {
    let facts = facts!("lambda z: z + y\n");
    assert!(facts.binds.is_empty());
    assert_eq!(names(&facts.reads), ["y"]);
}

#[test]
fn augmented_assign_target_is_also_a_read() {
    let facts = facts!("total += i\n");
    assert_eq!(names(&facts.binds), ["total"]);
    let mut reads = names(&facts.reads);
    reads.sort_unstable();
    assert_eq!(reads, ["i", "total"]);
    assert!(facts.mutates.iter().any(|name| &**name == "total"));
}

#[test]
fn walrus_binds_into_the_enclosing_scope() {
    let facts = facts!("(w := 1)\n");
    assert_eq!(names(&facts.binds), ["w"]);
}

#[test]
fn except_as_binds_and_deletes() {
    let facts = facts!("try:\n    pass\nexcept ValueError as e:\n    pass\n");
    assert!(facts.binds.iter().any(|name| &**name == "e"));
    assert!(facts.deletes.iter().any(|name| &**name == "e"));
}

#[test]
fn with_as_binds() {
    let facts = facts!("with open(p) as fh:\n    pass\n");
    assert!(facts.binds.iter().any(|name| &**name == "fh"));
    assert_eq!(facts.effect, Effect::ExternalRead);
}

#[test]
fn global_write_is_summarized() {
    let facts = facts!("def g():\n    global c\n    c = 1\n");
    let summary = facts.summary.expect("def has a summary");
    assert_eq!(names(&summary.global_writes), ["c"]);
}

#[test]
fn free_name_mutation_is_summarized() {
    let facts = facts!("def register(f):\n    routes.append(f)\n    return f\n");
    let summary = facts.summary.expect("def has a summary");
    assert_eq!(names(&summary.mutates_frees), ["routes"]);
    assert_eq!(summary.mutates_params, [0]);
}

#[test]
fn callees_are_recorded_for_transitive_closure() {
    let facts = facts!("def h():\n    g()\n");
    let summary = facts.summary.expect("def has a summary");
    assert!(summary.callees.iter().any(|name| &**name == "g"));
}

#[test]
fn inheritance_is_an_alias_edge() {
    let facts = facts!("class C(B):\n    pass\n");
    assert!(
        facts
            .alias_edges
            .iter()
            .any(|(a, b)| &**a == "C" && &**b == "B")
    );
}

#[test]
fn bare_name_assignment_is_an_alias_edge() {
    let facts = facts!("b = a\n");
    assert!(
        facts
            .alias_edges
            .iter()
            .any(|(x, y)| &**x == "b" && &**y == "a")
    );
}

#[test]
fn passing_an_argument_makes_it_a_mutation_candidate() {
    let facts = facts!("add(a)\n");
    assert!(facts.mutates.iter().any(|name| &**name == "a"));
    assert!(facts.calls.iter().any(|name| &**name == "add"));
}

#[test]
fn pure_whitelist_calls_do_not_mutate_arguments() {
    let facts = facts!("n = len(a)\n");
    assert!(!facts.mutates.iter().any(|name| &**name == "a"));
}

#[test]
fn method_calls_mutate_the_receiver() {
    let facts = facts!("keep.append(1)\n");
    assert!(facts.mutates.iter().any(|name| &**name == "keep"));
}

#[test]
fn attribute_and_subscript_stores_mutate_the_root() {
    assert!(
        facts!("config.limit = 99\n")
            .mutates
            .iter()
            .any(|name| &**name == "config")
    );
    assert!(
        facts!("rows[0] = 1\n")
            .mutates
            .iter()
            .any(|name| &**name == "rows")
    );
    assert!(
        facts!("del rows[0]\n")
            .mutates
            .iter()
            .any(|name| &**name == "rows")
    );
}

#[test]
fn del_records_a_delete() {
    let facts = facts!("del x\n");
    assert_eq!(names(&facts.deletes), ["x"]);
}

#[test]
fn decorators_are_calls() {
    let facts = facts!("@register\ndef hello():\n    pass\n");
    assert!(facts.calls.iter().any(|name| &**name == "register"));
    assert!(facts.binds.iter().any(|name| &**name == "hello"));
}

#[test]
fn conditional_import_binds_the_union() {
    let facts = facts!(
        "try:\n    import cupy as xp\nexcept ImportError:\n    import numpy as xp\n",
    );
    assert_eq!(names(&facts.binds), ["xp"]);
}

#[test]
fn dotted_import_binds_the_root() {
    let facts = facts!("import os.path\n");
    assert_eq!(names(&facts.binds), ["os"]);
    assert_eq!(facts.effect, Effect::Import);
}

#[test]
fn for_loops_bind_the_target_and_consume_the_iterable() {
    let facts = facts!("for row in rows:\n    out.append(row)\n");
    assert!(facts.binds.iter().any(|name| &**name == "row"));
    assert!(facts.mutates.iter().any(|name| &**name == "rows"));
    assert!(facts.mutates.iter().any(|name| &**name == "out"));
}

#[test]
fn reflective_constructs_are_opaque() {
    for code in [
        "exec(code)\n",
        "globals().update(d)\n",
        "setattr(o, name, v)\n",
        "delattr(o, name)\n",
        "eval(expr)\n",
        "vars(m)['x'] = 1\n",
        "locals()\n",
        "__import__('os')\n",
        "import importlib\n",
        "getattr(o, name)\n",
        "o.__dict__['x'] = 1\n",
        "f.__globals__['x'] = 1\n",
        "sys.modules['m'] = fake\n",
        "from m import *\n",
        "g = globals\n",
    ] {
        let facts = facts!(code);
        assert!(facts.opaque, "expected opaque: {code}");
        assert_eq!(facts.effect, Effect::Opaque, "expected Opaque effect: {code}");
    }
}

#[test]
fn getattr_with_a_literal_attribute_is_not_reflective() {
    let facts = facts!("v = getattr(o, 'name')\n");
    assert!(!facts.opaque);
}

#[test]
fn a_def_whose_body_is_reflective_is_opaque_to_call() {
    let facts = facts!("def f():\n    exec(code)\n");
    assert!(facts.opaque);
    assert!(facts.summary.expect("summary").opaque);
}

#[test]
fn nested_defs_are_absorbed_into_the_summary() {
    let facts = facts!("def outer():\n    def inner():\n        global c\n        c = 1\n    inner()\n");
    let summary = facts.summary.expect("summary");
    assert_eq!(names(&summary.global_writes), ["c"]);
}

#[test]
fn named_lambda_gets_a_summary() {
    let facts = facts!("f = lambda: routes.append(1)\n");
    let summary = facts.summary.expect("named lambda has a summary");
    assert_eq!(names(&summary.mutates_frees), ["routes"]);
}

#[test]
fn loose_lambda_effects_are_absorbed_into_the_statement() {
    let facts = facts!("subscribe(lambda: routes.append(1))\n");
    assert!(facts.mutates.iter().any(|name| &**name == "routes"));
}

#[test]
fn class_bodies_execute_now() {
    let facts = facts!("class C:\n    y = helper()\n");
    assert!(facts.calls.iter().any(|name| &**name == "helper"));
    assert_eq!(names(&facts.binds), ["C"]);
}

#[test]
fn class_methods_fold_into_the_class_summary() {
    let facts = facts!("class C:\n    def bump(self):\n        global c\n        c = 1\n");
    let summary = facts.summary.expect("class has a summary");
    assert_eq!(names(&summary.global_writes), ["c"]);
}

#[test]
fn effects_are_classified() {
    assert_eq!(facts!("x = 1\n").effect, Effect::Pure);
    assert_eq!(facts!("import pandas as pd\n").effect, Effect::Import);
    assert_eq!(facts!("print(x)\n").effect, Effect::Output);
    assert_eq!(facts!("df = pd.read_csv('a.csv')\n").effect, Effect::ExternalRead);
    assert_eq!(facts!("df.to_csv('out.csv')\n").effect, Effect::ExternalWrite);
    assert_eq!(facts!("x = random.random()\n").effect, Effect::Nondeterministic);
}

#[test]
fn compound_statements_union_their_branches() {
    let facts = facts!("if flag:\n    x = 1\nelse:\n    y = 2\n");
    assert!(facts.binds.iter().any(|name| &**name == "x"));
    assert!(facts.binds.iter().any(|name| &**name == "y"));
    assert!(facts.reads.iter().any(|name| &**name == "flag"));
}

#[test]
fn match_patterns_bind_their_captures() {
    let facts = facts!(
        "match point:\n    case (x, y):\n        pass\n    case {**rest}:\n        pass\n",
    );
    assert!(facts.binds.iter().any(|name| &**name == "x"));
    assert!(facts.binds.iter().any(|name| &**name == "y"));
    assert!(facts.binds.iter().any(|name| &**name == "rest"));
}

#[test]
fn walrus_leaks_out_of_comprehensions_but_targets_do_not() {
    let facts = facts!("[y := q for q in range(3)]\n");
    assert!(facts.binds.iter().any(|name| &**name == "y"));
    assert!(!facts.binds.iter().any(|name| &**name == "q"));
}

#[test]
fn attribute_augassign_mutates_the_root() {
    let facts = facts!("x.y += 1\n");
    assert!(facts.mutates.iter().any(|name| &**name == "x"));
}

#[test]
fn starred_and_keyword_arguments_are_mutation_candidates() {
    assert!(
        facts!("f(*items)\n")
            .mutates
            .iter()
            .any(|name| &**name == "items")
    );
    assert!(
        facts!("f(data=rows)\n")
            .mutates
            .iter()
            .any(|name| &**name == "rows")
    );
}

#[test]
fn nested_unpacking_binds_every_target() {
    let facts = facts!("(a, b), c = pair\n");
    for name in ["a", "b", "c"] {
        assert!(facts.binds.iter().any(|bind| &**bind == name));
    }
    assert!(facts.reads.iter().any(|name| &**name == "pair"));
}

#[test]
fn global_del_in_a_class_body_deletes_now() {
    let facts = facts!("class C:\n    global g\n    del g\n");
    assert!(facts.deletes.iter().any(|name| &**name == "g"));
}

#[test]
fn mentions_cover_every_name() {
    let facts = facts!("def f(a):\n    return helper(a) + y\n");
    for name in ["f", "helper", "y"] {
        assert!(
            facts.mentions.iter().any(|mention| &**mention == name),
            "missing mention: {name}"
        );
    }
}
