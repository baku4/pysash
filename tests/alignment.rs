//! Alignment counterexamples, invariants, and edit-loop convergence.

use pysash::source::PythonSource;
use pysash::SessionHistory;
use pysash::plan::{Action, DecisionReason, SessionDiagnostic, StatementDiagnostic};
use Action::{Reuse, Run};

fn source(text: &str) -> PythonSource {
    PythonSource::parse(text).expect("valid python")
}

fn session(pushed: &[&str]) -> SessionHistory {
    let mut history = SessionHistory::new();
    for text in pushed {
        history.push(&source(text));
    }
    history
}

fn actions(history: &SessionHistory, code: &str) -> Vec<Action> {
    history
        .align(&source(code))
        .steps
        .iter()
        .map(|plan| plan.action)
        .collect()
}

fn reasons(history: &SessionHistory, code: &str) -> Vec<DecisionReason> {
    history
        .align(&source(code))
        .steps
        .iter()
        .map(|plan| plan.reason.clone())
        .collect()
}

fn nothing_to_run(history: &SessionHistory, code: &str) -> bool {
    history.align(&source(code)).run_steps().next().is_none()
}

/// A pure session prefix is reused and only the source suffix runs.

#[test]
fn a_session_that_is_a_prefix_is_reused() {
    let history = session(&["import os\n", "x = 1\n"]);
    assert_eq!(
        actions(&history, "import os\nx = 1\ny = x\n"),
        [Reuse, Reuse, Run]
    );
    assert_eq!(
        reasons(&history, "import os\nx = 1\ny = x\n")[2],
        DecisionReason::NoMatchingExecution
    );
}

#[test]
fn re_aligning_the_same_source_reuses_everything() {
    let history = session(&["import os\n", "x = 1\nprint(x)\n"]);
    assert!(nothing_to_run(&history, "import os\nx = 1\nprint(x)\n"));
    let plan = history.align(&source("import os\nx = 1\nprint(x)\n"));
    assert_eq!(plan.summary().reused, 3);
    assert_eq!(plan.residue_len, 0);
}

#[test]
fn trivia_does_not_break_reuse() {
    let history = session(&["x = 1000\n"]);
    assert!(nothing_to_run(&history, "x=1_000  # comment\n"));
}

#[test]
fn an_empty_session_runs_everything() {
    let history = SessionHistory::new();
    assert_eq!(actions(&history, "x = 1\n"), [Run]);
    assert_eq!(
        reasons(&history, "x = 1\n"),
        [DecisionReason::NoMatchingExecution]
    );
}

#[test]
fn a_reordered_source_has_no_prefix_and_runs_entirely() {
    let history = session(&["x = 1\n", "y = x\n"]);
    let plan = history.align(&source("y = x\nx = 1\n"));
    assert_eq!(plan.prefix_len, 0);
    assert!(plan.steps.iter().all(|p| p.action == Run));
}

/// A prefix execution is not reusable after a later execution disturbs its result.

#[test]
fn late_binding_reruns_the_clobbered_binding() {
    // The old call read K=20; the new call must read K=10.
    let history = session(&[
        "K = 10\n",
        "def f():\n    return K * 2\n",
        "K = 20\n",
        "y = f()\n",
    ]);
    let code = "K = 10\ndef f():\n    return K * 2\ny = f()\n";
    assert_eq!(actions(&history, code), [Run, Reuse, Run]);
    assert_eq!(
        reasons(&history, code)[0],
        DecisionReason::BindingChanged { name: "K".into() }
    );
}

#[test]
fn argument_mutation_reruns_the_mutated_producer() {
    // `add(a)` mutated `a`, so the source must recreate the empty list.
    let history = session(&[
        "a = []\n",
        "def add(l):\n    l.append(1)\n",
        "add(a)\n",
        "n = len(a)\n",
    ]);
    let code = "a = []\ndef add(l):\n    l.append(1)\nn = len(a)\n";
    assert_eq!(actions(&history, code), [Run, Reuse, Run]);
    assert_eq!(
        reasons(&history, code)[0],
        DecisionReason::DependencyChanged { name: "a".into() }
    );
}

#[test]
fn decorator_registry_mutation_is_transitive() {
    // `@register` appended to `routes`, so the source must recreate the registry.
    let history = session(&[
        "routes = []\n",
        "def register(f):\n    routes.append(f)\n    return f\n",
        "@register\ndef hello():\n    pass\n",
        "n = len(routes)\n",
    ]);
    let code = "routes = []\ndef register(f):\n    routes.append(f)\n    return f\nn = len(routes)\n";
    assert_eq!(actions(&history, code), [Run, Reuse, Run]);
}

#[test]
fn transitive_global_writes_rerun_the_binding() {
    // `h()` calls `g()`, which rebinds global `c`.
    let history = session(&[
        "def g():\n    global c\n    c = 1\n",
        "def h():\n    g()\n",
        "c = 0\n",
        "h()\n",
        "z = c\n",
    ]);
    let code = "def g():\n    global c\n    c = 1\ndef h():\n    g()\nc = 0\nz = c\n";
    assert_eq!(actions(&history, code), [Reuse, Reuse, Run, Run]);
    assert_eq!(
        reasons(&history, code)[2],
        DecisionReason::BindingChanged { name: "c".into() }
    );
}

#[test]
fn repeated_statements_are_distinguished_by_position() {
    // Two positional occurrences of `add(a)` represent two executions.
    let history = session(&[
        "a = []\n",
        "def add(l):\n    l.append(1)\n",
        "add(a)\n",
        "n = len(a)\n",
    ]);
    let code = "a = []\ndef add(l):\n    l.append(1)\nadd(a)\nadd(a)\nn = len(a)\n";
    assert_eq!(actions(&history, code), [Reuse, Reuse, Reuse, Run, Run]);
}

#[test]
fn container_aliases_poison_the_original_name() {
    // Mutating `keep` after `keep = a` also mutates `a`.
    let history = session(&["a = []\n", "keep = a\n", "keep.append(1)\n"]);
    let code = "a = []\n";
    assert_eq!(actions(&history, code), [Run]);
    // Either alias may identify the same disturbed object.
    assert!(matches!(
        reasons(&history, code)[0],
        DecisionReason::DependencyChanged { .. }
    ));
}

#[test]
fn inheritance_shares_mutable_attributes() {
    // `C.items.append(1)` mutates the attribute inherited from `B`.
    let history = session(&[
        "class B:\n    items = []\n",
        "class C(B):\n    pass\n",
        "C.items.append(1)\n",
        "n = len(B.items)\n",
    ]);
    let code = "class B:\n    items = []\nclass C(B):\n    pass\nn = len(B.items)\n";
    assert_eq!(actions(&history, code), [Run, Run, Run]);
}

#[test]
fn attribute_mutation_does_not_poison_unrelated_bindings() {
    // Rebinding `config.LIMIT` does not alter the previously imported `LIMIT` name.
    let history = session(&[
        "import config\n",
        "from config import LIMIT\n",
        "config.LIMIT = 99\n",
    ]);
    let code = "import config\nfrom config import LIMIT\n";
    // Only the disturbed binding runs; `Run` need not be contiguous.
    assert_eq!(actions(&history, code), [Run, Reuse]);
}

#[test]
fn reflective_residue_runs_everything() {
    let history = session(&[
        "import builtins\n",
        "builtins.len = str\n",
        "n = len('abc')\n",
    ]);
    let code = "import builtins\nn = len('abc')\n";
    let plan = history.align(&source(code));
    assert!(plan.steps.iter().all(|p| p.action == Run));

    // The diagnostic carries retained source text rather than an obsolete source index.
    let Some(SessionDiagnostic::OpaqueResidue { text }) = plan.diagnostics.first() else {
        panic!("expected reflective residue");
    };
    assert_eq!(&**text, "builtins.len = str");
}

#[test]
fn a_diverged_session_reports_its_residue() {
    let history = session(&["x = 1\n", "y = 2\n"]);
    let plan = history.align(&source("x = 1\n"));
    // `residue_len` reports divergence without a duplicate diagnostic.
    assert_eq!(plan.residue_len, 1);
    assert!(plan.diagnostics.is_empty());
}

#[test]
fn poison_runs_everything_forever() {
    let mut history = session(&["x = 1\n"]);
    history.poison();
    assert_eq!(actions(&history, "x = 1\n"), [Run]);
    assert_eq!(
        reasons(&history, "x = 1\n"),
        [DecisionReason::NoMatchingExecution]
    );
}

/// Partial execution invalidates only the effects it may have produced.

#[test]
fn an_interrupted_source_disturbs_what_it_might_have_bound() {
    // The interpreter may hold either value, so the old `x = 1` cannot remain a witness.
    let mut history = SessionHistory::new();
    history.realize(&source("x = 1\n"));
    history.record_partial(&source("x = 2\nboom()\n"));

    let code = "x = 1\ny = x\n";
    assert_eq!(actions(&history, code), [Run, Run]);
    assert_eq!(
        reasons(&history, code)[0],
        DecisionReason::BindingChanged { name: "x".into() }
    );
}

#[test]
fn an_interrupted_source_leaves_untouched_names_reusable() {
    // Unlike poisoning, partial recording preserves names the source could not touch.
    let mut history = SessionHistory::new();
    history.realize(&source("import os\nbig = [1, 2, 3]\n"));
    history.record_partial(&source("total = 0\nboom()\n"));
    assert_eq!(
        actions(&history, "import os\nbig = [1, 2, 3]\nn = 3\n"),
        [Reuse, Reuse, Run]
    );
}

#[test]
fn an_interrupted_source_is_never_itself_reused() {
    // Residue is not a reuse witness, even for identical source.
    let mut history = SessionHistory::new();
    history.record_partial(&source("x = 1\ny = x\n"));
    assert_eq!(actions(&history, "x = 1\ny = x\n"), [Run, Run]);
    assert_eq!(history.statement_count(), 0);
    // Without a realized witness, this residue cannot affect any verdict and is discarded.
    assert_eq!(history.residue_count(), 0);
}

#[test]
fn an_interrupted_run_converges_once_it_is_fixed() {
    // A known partial execution remains recoverable.
    let mut history = SessionHistory::new();
    history.realize(&source("x = 1\n"));
    history.record_partial(&source("x = 2\nboom()\n"));

    let fixed = source("x = 1\nx = 2\ny = x\n");
    history.realize(&fixed);
    assert!(nothing_to_run(&history, "x = 1\nx = 2\ny = x\n"));
}

/// Executing a plan and realizing it converges immediately.

#[test]
fn the_edit_loop_converges_by_realizing() {
    let mut history = session(&["import math\n", "r = 2.0\n"]);
    let edited = source("import math\nr = 3.0\narea = math.pi * r ** 2\n");

    let plan = history.align(&edited);
    let acts: Vec<Action> = plan.steps.iter().map(|p| p.action).collect();
    // No later execution disturbed `math`, so the import remains reusable.
    assert_eq!(acts, [Reuse, Run, Run]);
    assert_eq!(plan.steps[1].reason, DecisionReason::StatementChanged);

    history.realize(&edited);
    let plan = history.align(&edited);
    assert!(plan.run_steps().next().is_none());
    assert_eq!(plan.summary().reused, 3);
    // The displaced `r = 2.0` remains as residue.
    assert_eq!(history.residue_count(), 1);
}

#[test]
fn disturbed_prefix_runs_converge_after_realize() {
    // Realizing a rerun gives it a later sequence than the residue that disturbed it.
    let mut history = session(&[
        "K = 10\n",
        "def f():\n    return K * 2\n",
        "K = 20\n",
        "y = f()\n",
    ]);
    let code = source("K = 10\ndef f():\n    return K * 2\ny = f()\n");
    assert_eq!(
        history.align(&code).steps.iter().map(|p| p.action).collect::<Vec<_>>(),
        [Run, Reuse, Run]
    );
    history.realize(&code);
    assert!(nothing_to_run(&history, "K = 10\ndef f():\n    return K * 2\ny = f()\n"));
}

#[test]
fn a_rerun_definition_replaces_the_stale_summary() {
    // A rerun definition must supersede an intervening stale definition.
    let mut history = session(&[
        "def f():\n    global c\n    c = 1\n",
        "c = 0\n",
        "def f():\n    pass\n",
    ]);
    let restored = source("def f():\n    global c\n    c = 1\nc = 0\nf()\n");
    history.realize(&restored); // The first definition runs again in place.
    history.push(&source("c = 99\n"));

    // `c = 99` overwrote the value produced by `f()`, so the call must run again.
    let code = "def f():\n    global c\n    c = 1\nc = 0\nf()\nz = c\n";
    assert_eq!(actions(&history, code), [Reuse, Run, Run, Run]);
    assert_eq!(
        reasons(&history, code)[2],
        DecisionReason::BindingChanged { name: "c".into() }
    );
}

#[test]
fn realize_does_not_let_old_executions_poison_new_ones() {
    // An older execution cannot disturb one that occurred later.
    let mut history = session(&["x = 1\n", "y = x + 1\n"]);
    let edited = source("x = 2\ny = x + 1\n");
    history.realize(&edited);
    assert!(nothing_to_run(&history, "x = 2\ny = x + 1\n"));
}

#[test]
fn growing_the_source_converges_with_realize() {
    let mut history = SessionHistory::new();
    let first = source("import math\nr = 2.0\n");
    history.realize(&first);
    assert!(nothing_to_run(&history, "import math\nr = 2.0\n"));

    let grown = source("import math\nr = 2.0\narea = math.pi * r ** 2\n");
    let plan = history.align(&grown);
    assert_eq!(plan.summary().reused, 2);
    history.realize(&grown);
    assert!(nothing_to_run(&history, "import math\nr = 2.0\narea = math.pi * r ** 2\n"));
    assert_eq!(history.residue_count(), 0);
}

#[test]
fn accumulation_is_never_double_counted_or_wrongly_skipped() {
    let history = session(&["acc = 0\n", "acc = acc + 1\n"]);
    // Appending one increment runs only that new increment.
    assert_eq!(
        actions(&history, "acc = 0\nacc = acc + 1\nacc = acc + 1\n"),
        [Reuse, Reuse, Run]
    );
    // Returning to the initial source must reset the already incremented accumulator.
    assert_eq!(actions(&history, "acc = 0\n"), [Run]);
}

/// Structural alignment invariants.

#[test]
fn reuse_never_appears_beyond_the_prefix() {
    let fixtures: &[(&[&str], &str)] = &[
        (&["x = 1\n", "y = x\n"], "x = 1\ny = x\nz = y\n"),
        (&["a = []\n", "a.append(1)\n"], "a = []\n"),
        (&["K = 10\n", "K = 20\n"], "K = 10\nz = K\n"),
        (&["import os\n"], "import sys\nimport os\n"),
    ];
    for (pushed, code) in fixtures {
        let plan = session(pushed).align(&source(code));
        for statement in &plan.steps {
            if statement.action == Reuse {
                assert!(
                    statement.index < plan.prefix_len,
                    "Reuse beyond prefix: {code:?}"
                );
            }
        }
    }
}

#[test]
fn pushes_concatenate_into_one_linear_trace() {
    let history = session(&["x = 1\n", "y = x\nz = y\n"]);
    assert_eq!(actions(&history, "x = 1\ny = x\nz = y\n"), [Reuse, Reuse, Reuse]);
}

#[test]
fn pushes_from_several_sources_form_one_realized_sequence() {
    let history = session(&["import os\n", "x = 1\ny = 2\n"]);
    assert_eq!(history.statement_count(), 3);
    assert!(nothing_to_run(&history, "import os\nx = 1\ny = 2\n"));
}

#[test]
fn live_names_track_binds_and_deletes() {
    let history = session(&["x = 1\n", "y = 2\n", "del x\n"]);
    let live: Vec<&str> = history.live_names().collect();
    assert_eq!(live, ["y"]);
}

#[test]
fn session_only_names_are_flagged_as_unresolved() {
    let history = session(&["df = 1\n"]);
    let plan = history.align(&source("out = df + 1\n"));
    // `df` is session-relative because this source never binds it.
    assert!(plan.steps[0].diagnostics.iter().any(
        |d| matches!(d, StatementDiagnostic::UnresolvedReference { name } if &**name == "df")
    ));
}

#[test]
fn self_contained_sources_have_no_unresolved_reads() {
    let history = SessionHistory::new();
    let plan = history.align(&source("import os\nx = 1\ny = x + len(os.sep)\nprint(y)\n"));
    assert!(plan.steps.iter().all(|p| p.diagnostics.is_empty()));
}

#[test]
fn downgrade_from_turns_the_tail_into_run() {
    let history = session(&["x = 1\n", "y = 2\n", "z = 3\n"]);
    let mut plan = history.align(&source("x = 1\ny = 2\nz = 3\n"));
    plan.downgrade_from(1);
    let acts: Vec<Action> = plan.steps.iter().map(|p| p.action).collect();
    assert_eq!(acts, [Reuse, Run, Run]);
    assert_eq!(plan.summary().reused, 1);
    assert_eq!(plan.summary().run, 2);
    assert_eq!(plan.summary().first_run, Some(1));
    // Downgrading preserves the original reuse reason.
    assert_eq!(plan.steps[1].reason, DecisionReason::ReusableExecution);
}
