//! Alignment의 완료 판정 — 잘못된 재사용을 낳던 실측 반례들과 편집 루프의 수렴.

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

/// 세션이 소스의 순수 prefix면 그만큼 재사용하고 나머지만 실행한다.

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

/// 편집 지점 위라도, 그 실행이 남긴 것을 뒤의 실행이 건드렸으면 재사용할 수 없다.

#[test]
fn late_binding_reruns_the_clobbered_binding() {
    // 실측: H에서 y = f()는 K=20을 읽어 40. P에서 f()는 K=10을 읽어 20이어야 한다.
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
    // 실측: add(a)가 a를 in-place로 바꿨다. P의 a = []는 다시 만들어져야 한다.
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
    // 실측: @register가 routes.append를 호출했다. routes = []는 다시 만들어져야 한다.
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
    // 실측: h() → g() → global c 재바인딩. c = 0은 다시 실행되어야 한다.
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
    // 실측: add(a)를 두 번 쓰면 두 번 실행되어야 한다 (n == 2).
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
    // 실측: keep = a 후 keep.append(1)은 a를 바꾼 것이다.
    let history = session(&["a = []\n", "keep = a\n", "keep.append(1)\n"]);
    let code = "a = []\n";
    assert_eq!(actions(&history, code), [Run]);
    // 이름은 a일 수도(원본), keep일 수도(별칭) 있다 — 같은 객체다.
    assert!(matches!(
        reasons(&history, code)[0],
        DecisionReason::DependencyChanged { .. }
    ));
}

#[test]
fn inheritance_shares_mutable_attributes() {
    // 실측: C.items.append(1)은 B.items를 바꾼 것이다 (상속 별칭).
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
    // 실측: config.LIMIT = 99는 이미 복사된 LIMIT 바인딩을 바꾸지 못한다.
    let history = session(&[
        "import config\n",
        "from config import LIMIT\n",
        "config.LIMIT = 99\n",
    ]);
    let code = "import config\nfrom config import LIMIT\n";
    // Run이 연속 구간이 아니다 — 오염된 것만 골라서 다시 돈다.
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

    // 반사적 구문은 입력 소스가 아니라 세션이 과거에 받은 소스에 있다 — source
    // 인덱스가 함께 와야 원문을 짚을 수 있다.
    let Some(SessionDiagnostic::OpaqueResidue { source, range }) = plan.diagnostics.first() else {
        panic!("반사적 구문이 실현 밖에 있다");
    };
    assert_eq!(history.sources()[*source].slice(*range), b"builtins.len = str");
}

#[test]
fn a_diverged_session_reports_its_residue() {
    let history = session(&["x = 1\n", "y = 2\n"]);
    let plan = history.align(&source("x = 1\n"));
    // 갈라졌다는 사실은 residue_len이 말한다. 진단으로 한 번 더 말하지 않는다 —
    // 같은 것을 두 군데서 관리하게 되기 때문이다.
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

/// 편집 루프 — 실행하고 realize하면 그 자리에서 수렴한다.

#[test]
fn the_edit_loop_converges_by_realizing() {
    let mut history = session(&["import math\n", "r = 2.0\n"]);
    let edited = source("import math\nr = 3.0\narea = math.pi * r ** 2\n");

    let plan = history.align(&edited);
    let acts: Vec<Action> = plan.steps.iter().map(|p| p.action).collect();
    // import는 재사용된다 — 뒤의 어떤 실행도 math를 건드리지 않았다.
    assert_eq!(acts, [Reuse, Run, Run]);
    assert_eq!(plan.steps[1].reason, DecisionReason::StatementChanged);

    history.realize(&edited);
    let plan = history.align(&edited);
    assert!(plan.run_steps().next().is_none());
    assert_eq!(plan.summary().reused, 3);
    // 밀려난 r = 2.0 하나가 residue로 남는다.
    assert_eq!(history.residue_count(), 1);
}

#[test]
fn disturbed_prefix_runs_converge_after_realize() {
    // K = 10은 오염 때문에 Run이었다. realize가 그 자리를 새 실행으로 바꿔 달지
    // 않으면, 이미 지나간 K = 20이 영원히 그 자리를 Run으로 만든다.
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
fn realize_does_not_let_old_executions_poison_new_ones() {
    // 옛 실행은 새 실행보다 먼저 일어났다 — 시간을 거슬러 오염시키지 못한다.
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
    // 뒤에 한 번 더 붙이면 그 한 번만 실행된다.
    assert_eq!(
        actions(&history, "acc = 0\nacc = acc + 1\nacc = acc + 1\n"),
        [Reuse, Reuse, Run]
    );
    // 처음으로 돌아가려면 acc = 0을 다시 실행해야 한다 — 세션의 acc는 이미 1이다.
    assert_eq!(actions(&history, "acc = 0\n"), [Run]);
}

/// 구조 성질들.

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
fn the_session_keeps_each_source_it_was_given() {
    let history = session(&["import os\n", "x = 1\ny = 2\n"]);
    assert_eq!(history.sources().len(), 2);
    assert_eq!(history.sources()[0].statements().len(), 1);
    assert_eq!(history.sources()[1].statements().len(), 2);
    assert_eq!(history.sources()[0].raw(), b"import os\n");
    assert_eq!(history.statement_count(), 3);
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
    // df는 이 소스 어디에서도 바인딩되지 않는다 — 세션에서는 돌지만 fresh run에서는
    // 재현되지 않는 조각이라는 신호다.
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
    // 판정의 기록은 남는다 — 재사용 가능했지만 호출자가 내렸다.
    assert_eq!(plan.steps[1].reason, DecisionReason::ReusableExecution);
}
