//! 재사용 판정. 잘못된 재사용은 조용히 틀린 결과이므로, "재사용하지 않는다"를
//! 확인하는 테스트가 더 중요하다.

use pysash::python_source::PythonSource;
use pysash::{Action, DecisionReason, SessionHistory};

fn source(text: &str) -> PythonSource {
    PythonSource::parse(text).unwrap_or_else(|e| panic!("{text:?}: {e}"))
}

fn session(pushed: &[&str]) -> SessionHistory {
    let mut history = SessionHistory::new();
    for text in pushed {
        history.push(&source(text));
    }
    history
}

/// 다시 실행하지 않아도 되는 statement 수.
fn reused(plan: &pysash::AlignmentPlan) -> usize {
    plan.plans.len() - plan.run_plans().count()
}

/// 할 일이 하나도 없다.
fn nothing_to_run(plan: &pysash::AlignmentPlan) -> bool {
    plan.run_plans().next().is_none()
}

fn actions(history: &SessionHistory, code: &str) -> Vec<Action> {
    history
        .align(&source(code))
        .plans
        .iter()
        .map(|plan| plan.action)
        .collect()
}

fn reasons(history: &SessionHistory, code: &str) -> Vec<DecisionReason> {
    history
        .align(&source(code))
        .plans
        .iter()
        .map(|plan| plan.reason)
        .collect()
}

/// 세션의 끝이 이 소스의 앞과 이어지면 그만큼은 방금 이 소스를 실행한 것이다.
#[test]
fn a_session_that_is_a_prefix_is_reused() {
    let history = session(&["x = 1\ny = 2\n"]);

    assert_eq!(
        actions(&history, "x = 1\ny = 2\nz = 3\n"),
        [Action::Reuse, Action::Reuse, Action::Run]
    );
    assert_eq!(
        reasons(&history, "x = 1\ny = 2\nz = 3\n"),
        [
            DecisionReason::ReusableExecution,
            DecisionReason::ReusableExecution,
            DecisionReason::NoMatchingExecution,
        ]
    );
}

/// 닻은 세션의 **끝**이다. 앞에 무엇이 있었든 상관없다 — REPL에서 이것저것
/// 해보다가 스크립트를 붙여넣는 것이 이 도구의 주 용도다.
#[test]
fn the_anchor_is_the_tail_of_the_session() {
    let history = session(&["w = 0\nx = 1\ny = 2\n"]);

    assert_eq!(
        actions(&history, "x = 1\ny = 2\nz = 3\n"),
        [Action::Reuse, Action::Reuse, Action::Run]
    );
}

#[test]
fn re_aligning_the_same_source_reuses_everything() {
    let history = session(&["x = 1\ny = 2\n"]);
    let plan = history.align(&source("x = 1\ny = 2\n"));

    assert!(nothing_to_run(&plan));
    assert_eq!(reused(&plan), 2);
}

/// 공백과 주석은 statement의 정체성이 아니므로 재사용을 막지 않는다.
#[test]
fn trivia_does_not_break_reuse() {
    let history = session(&["x = 1000\n"]);

    assert!(nothing_to_run(
        &history.align(&source("x = 1_000  # comment\n"))
    ));
}

#[test]
fn an_empty_session_runs_everything() {
    let history = session(&[]);

    assert_eq!(actions(&history, "x = 1\n"), [Action::Run]);
    assert_eq!(
        reasons(&history, "x = 1\n"),
        [DecisionReason::NoMatchingExecution]
    );
}

/// 세션의 끝이 이 소스의 앞과 이어지지 않으면 재사용이 없다. 되돌릴 수 없는 실행
/// 위에서 이보다 나은 답은 없다 — 이건 한계이지 버그가 아니다.
#[test]
fn a_source_that_does_not_continue_the_session_runs_entirely() {
    let history = session(&["x = 1\ny = 2\nz = 3\n"]);
    let plan = history.align(&source("x = 1\ny = 2\nz = 99\n"));

    assert_eq!(reused(&plan), 0);
    assert_eq!(
        reasons(&history, "x = 1\ny = 2\nz = 99\n"),
        [
            DecisionReason::NoMatchingExecution,
            DecisionReason::NoMatchingExecution,
            DecisionReason::StatementChanged,
        ]
    );
}

/// 순서가 바뀌어도 세션의 끝이 이어지면 그만큼은 재사용된다. 세션은 `[x=1, y=x]`를
/// 실행했으므로 `y`는 이미 `1`이고, 여기서 `x = 1`만 다시 실행하면 새 소스를
/// 통째로 실행한 것과 같은 상태가 된다.
#[test]
fn a_reordered_source_reuses_what_still_lines_up() {
    let history = session(&["x = 1\ny = x\n"]);

    assert_eq!(
        actions(&history, "y = x\nx = 1\n"),
        [Action::Reuse, Action::Run]
    );
}

/// 누산은 재사용이 건전성 문제가 되는 자리다.
#[test]
fn accumulation_is_never_double_counted_or_wrongly_skipped() {
    let history = session(&["acc = 0\nacc = acc + 1\n"]);

    // 세션의 끝이 이어진다 — 이미 실행된 두 줄은 재사용, 새 줄만 실행. acc는 2가 된다.
    assert_eq!(
        actions(&history, "acc = 0\nacc = acc + 1\nacc = acc + 1\n"),
        [Action::Reuse, Action::Reuse, Action::Run]
    );

    // 이어지지 않는다 — 재사용하면 acc가 1로 남아 틀린다. 다시 실행해야 한다.
    assert_eq!(actions(&history, "acc = 0\n"), [Action::Run]);
}

/// 같은 statement가 두 번 나와도 위치로 구분된다.
#[test]
fn repeated_statements_are_distinguished_by_position() {
    let history = session(&["a = []\na.append(1)\n"]);

    assert_eq!(
        actions(&history, "a = []\na.append(1)\na.append(1)\n"),
        [Action::Reuse, Action::Reuse, Action::Run]
    );
}

/// 여러 번 push한 것이 하나의 선형 열로 이어진다.
#[test]
fn pushes_concatenate_into_one_linear_trace() {
    let history = session(&["import os\n", "x = 1\ny = 2\n"]);

    assert_eq!(
        actions(&history, "import os\nx = 1\ny = 2\nz = 3\n"),
        [Action::Reuse, Action::Reuse, Action::Reuse, Action::Run]
    );
}

/// 문서의 트리가 보존된다 — statement를 평탄화해 버리지 않는다.
#[test]
fn the_session_keeps_each_source_it_was_given() {
    let history = session(&["import os\n", "x = 1\ny = 2\n"]);

    assert_eq!(history.sources().len(), 2);
    assert_eq!(history.sources()[0].statements().len(), 1);
    assert_eq!(history.sources()[1].statements().len(), 2);
    assert_eq!(history.sources()[0].raw(), b"import os\n");
}

/// 계획을 수행하고 **실행한 것만** 기록하면 루프가 그 자리에서 수렴한다.
/// 영구히 못 쓰게 되는 세션은 없다.
#[test]
fn the_edit_loop_converges_by_pushing_what_ran() {
    let mut history = session(&["x = 1\ny = 2\nz = 3\n"]);
    let edited = source("x = 1\ny = 2\nz = 99\n");

    assert_eq!(reused(&history.align(&edited)), 0);

    history.push(&edited);

    assert!(nothing_to_run(&history.align(&edited)));
}

/// Run은 언제나 소스의 뒤쪽 연속 구간이므로, 그 구간의 바이트만 잘라 실행하고
/// 그것만 기록할 수 있다.
#[test]
fn the_run_suffix_can_be_sliced_and_recorded() {
    let mut history = session(&["x = 1\n"]);
    let code = source("x = 1\ny = 2\nz = 3\n");

    let plan = history.align(&code);
    let first_run = plan.run_plans().next().expect("실행할 것이 있다");
    assert_eq!(first_run.index, 1);

    let tail = &code.raw()[first_run.range.start as usize..];
    assert_eq!(tail, b"y = 2\nz = 3\n");

    history.push(&source(str::from_utf8(tail).unwrap()));

    assert!(nothing_to_run(&history.align(&code)));
}

/// 알려진 한계: 잘라낸 조각의 첫 statement가 bare string이면 재파싱할 때
/// docstring으로 읽혀 정체성이 달라진다. 건전성 문제는 아니고 그 회차의 재사용만
/// 잃는다.
#[test]
fn slicing_a_run_suffix_that_starts_with_a_bare_string_loses_reuse() {
    let mut history = session(&["x = 1\n"]);
    let code = source("x = 1\n'note'\ny = 2\n");

    let first_run = history
        .align(&code)
        .run_plans()
        .next()
        .expect("실행할 것이 있다")
        .range;
    let tail = &code.raw()[first_run.start as usize..];
    history.push(&source(str::from_utf8(tail).unwrap()));

    assert_eq!(reused(&history.align(&code)), 0);
}
