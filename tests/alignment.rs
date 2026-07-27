//! 재사용 판정. 잘못된 재사용은 조용히 틀린 결과이므로, "재사용하지 않는다"를
//! 확인하는 테스트가 더 중요하다.

use pysash::{Action, DecisionReason};
use pysash::python_source::PythonSource;
use pysash::session_history::SessionHistory;

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

fn actions(history: &SessionHistory, code: &str) -> Vec<Action> {
    history
        .align(&source(code))
        .steps
        .iter()
        .map(|step| step.action)
        .collect()
}

fn reasons(history: &SessionHistory, code: &str) -> Vec<DecisionReason> {
    history
        .align(&source(code))
        .steps
        .iter()
        .map(|step| step.reason)
        .collect()
}

/// 세션이 입력 소스의 순수 prefix이면 그 앞부분은 문자 그대로 이 소스의 실행이다.
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

#[test]
fn re_aligning_the_same_source_reuses_everything() {
    let history = session(&["x = 1\ny = 2\n"]);
    let plan = history.align(&source("x = 1\ny = 2\n"));

    assert!(plan.is_full_reuse());
    assert_eq!(plan.reused_count(), 2);
    assert_eq!(plan.run_steps().count(), 0);
}

/// 공백과 주석은 statement의 정체성이 아니므로 재사용을 막지 않는다.
#[test]
fn trivia_does_not_break_reuse() {
    let history = session(&["x = 1000\n"]);
    let plan = history.align(&source("x = 1_000  # comment\n"));

    assert!(plan.is_full_reuse());
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

/// 세션은 linear하다. 순서가 바뀌면 공통 prefix가 없다.
#[test]
fn reordering_destroys_the_prefix() {
    let history = session(&["x = 1\ny = x\n"]);

    assert_eq!(
        actions(&history, "y = x\nx = 1\n"),
        [Action::Run, Action::Run]
    );
    assert_eq!(
        reasons(&history, "y = x\nx = 1\n"),
        [
            DecisionReason::StatementChanged,
            DecisionReason::NoMatchingExecution,
        ]
    );
}

/// 세션이 갈라지는 지점을 넘어 더 실행했다면, 그것이 무엇을 망가뜨렸는지 알 수
/// 없다. 앞부분까지 전부 다시 실행한다.
#[test]
fn a_session_that_ran_past_the_edit_point_forces_a_full_run() {
    let history = session(&["x = 1\ny = 2\nz = 3\n"]);
    let plan = history.align(&source("x = 1\ny = 2\nz = 99\n"));

    assert!(!plan.is_full_reuse());
    assert_eq!(plan.reused_count(), 0);
    assert_eq!(
        reasons(&history, "x = 1\ny = 2\nz = 99\n"),
        [
            DecisionReason::DependencyChanged,
            DecisionReason::DependencyChanged,
            DecisionReason::StatementChanged,
        ]
    );
}

/// 누산은 재사용이 건전성 문제가 되는 자리다. 세션이 `acc = acc + 1`을 이미
/// 실행했다면 다시 실행하면 안 되고, 세션이 더 나갔다면 재사용해서도 안 된다.
#[test]
fn accumulation_is_never_double_counted_or_wrongly_skipped() {
    let history = session(&["acc = 0\nacc = acc + 1\n"]);

    // 세션이 정확히 prefix다 — 이미 실행된 두 줄은 재사용, 새 줄만 실행.
    assert_eq!(
        actions(&history, "acc = 0\nacc = acc + 1\nacc = acc + 1\n"),
        [Action::Reuse, Action::Reuse, Action::Run]
    );

    // 세션이 소스보다 더 나갔다 — 되돌릴 수 없으므로 전부 실행.
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

/// 계획을 실행한 뒤 `realize`로 기록하면, 다시 정렬했을 때 아무것도 할 일이 없다.
#[test]
fn the_edit_loop_converges_after_realize() {
    let mut history = session(&["x = 1\n"]);
    let edited = source("x = 1\ny = 2\n");

    let plan = history.align(&edited);
    assert_eq!(plan.run_steps().count(), 1);

    history.realize(&edited);
    assert!(history.align(&edited).is_full_reuse());
}

/// `realize`가 밀어낸 잉여 실행은 계속 기억된다. 그 효과를 되돌릴 수 없기
/// 때문이다.
#[test]
fn displaced_statements_keep_disturbing_later_alignments() {
    let mut history = session(&["x = 1\ny = 2\n"]);
    let edited = source("x = 1\ny = 99\n");

    history.realize(&edited);

    // `y = 2`가 실현 열 밖으로 밀려났고 그 효과는 남아 있다.
    assert_eq!(
        actions(&history, "x = 1\ny = 99\n"),
        [Action::Run, Action::Run]
    );
}

/// 잉여 실행이 없는 `realize`는 세션을 깨끗하게 남긴다.
#[test]
fn realizing_an_extension_leaves_no_residue() {
    let mut history = session(&["x = 1\n"]);
    let grown = source("x = 1\ny = 2\nz = 3\n");

    history.realize(&grown);

    assert!(history.align(&grown).is_full_reuse());
    assert_eq!(
        actions(&history, "x = 1\ny = 2\nz = 3\nw = 4\n"),
        [Action::Reuse, Action::Reuse, Action::Reuse, Action::Run]
    );
}
