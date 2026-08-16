//! 실제 연구 코드에서 그대로 뽑아 온 fixture 위에서, 어떤 소스가 들어와도 참이어야
//! 하는 성질들.
//!
//! 여기에는 손으로 적은 기대값이 하나도 없다. 전부 판정 규칙 자체에서 따라 나오는
//! 것들이고, 그래서 fixture를 바꾸거나 늘려도 그대로 성립한다. 실제 코드로 도는
//! 이유는 하나다 — 장난감 예제에서는 절대 나오지 않는 모양(90줄짜리 plotting 함수
//! 하나가 statement 하나, `from … import *`, subprocess, 중첩 `with`)에서도
//! 성립해야 하기 때문이다.
//!
//! 특정 편집에 대한 구체적인 기대값은 `tests/editing.rs`에 있다.

mod support;

use pysash::SessionHistory;
use pysash::plan::Action;
use pysash::source::PythonSource;
use support::{actions, append_probe, corpus, head, insert_probe, nth, realized, replace_statement};
use Action::{Reuse, Run};

/// 삽입 지점을 코퍼스 크기에 맞춰 고른다 — 맨 앞, 앞쪽, 가운데, 맨 뒤.
fn edit_points(total: usize) -> Vec<usize> {
    assert!(total > 0, "statement가 없는 fixture에는 편집 지점이 없다");
    let mut points = vec![0, total / 3, total / 2, total - 1];
    points.sort_unstable();
    points.dedup();
    points
}

#[test]
fn every_corpus_file_is_parseable_python_with_statements() {
    for fixture in corpus() {
        assert!(
            !fixture.source.statements().is_empty(),
            "{}: statement가 하나도 없다",
            fixture.name
        );
        // 원본 바이트열은 준 그대로 남아 있어야 한다 — range가 이 바이트열 기준이다.
        assert_eq!(fixture.source.raw(), fixture.text.as_bytes(), "{}", fixture.name);
    }
}

/// range로 잘라낸 조각을 다시 파싱하면 같은 statement가 나와야 한다. 이게 깨지면
/// 호출자가 Run으로 잘라낸 코드를 인터프리터에 먹일 수 없다.
#[test]
fn every_statement_slices_back_to_the_same_statement() {
    for fixture in corpus() {
        for statement in fixture.source.statements() {
            let text = std::str::from_utf8(fixture.source.slice(statement.range))
                .expect("statement 경계는 문자 경계다");
            let reparsed = PythonSource::parse(text)
                .unwrap_or_else(|e| panic!("{}: {text:?}: {e}", fixture.name));
            assert_eq!(reparsed.statements().len(), 1, "{}: {text:?}", fixture.name);
            assert_eq!(
                reparsed.statements()[0].canonical,
                statement.canonical,
                "{}: {text:?}",
                fixture.name
            );
        }
    }
}

/// REPL에 그대로 밀어 넣은 소스를 다시 물으면 100% 재사용이다. 실현 밖 실행이
/// 없으므로 오염될 것도 없다.
#[test]
fn a_pushed_source_is_entirely_reusable() {
    for fixture in corpus() {
        let mut history = SessionHistory::new();
        history.push(&fixture.source);
        let plan = history.align(&fixture.source);

        assert_eq!(plan.summary().run, 0, "{}: {}", fixture.name, actions(&plan));
        assert_eq!(plan.residue_len, 0, "{}", fixture.name);
        assert!(plan.diagnostics.is_empty(), "{}", fixture.name);
    }
}

#[test]
fn a_realized_source_is_entirely_reusable() {
    for fixture in corpus() {
        let history = realized(&fixture.source);
        let plan = history.align(&fixture.source);
        assert_eq!(plan.summary().run, 0, "{}: {}", fixture.name, actions(&plan));
    }
}

/// align은 순수하다 — 세션을 바꾸지 않고, 두 번 물어도 같은 답이다.
#[test]
fn align_does_not_change_the_session() {
    for fixture in corpus() {
        let history = realized(&fixture.source);
        let before = history.clone();

        let first = history.align(&fixture.source);
        let second = history.align(&fixture.source);

        assert_eq!(first, second, "{}", fixture.name);
        assert!(history == before, "{}: align이 세션을 건드렸다", fixture.name);
    }
}

/// 세션이 소스의 순수 prefix면, 그 prefix는 통째로 재사용된다. 오염될 여지가
/// 아예 없다 — 실현 밖 실행이 하나도 없기 때문이다.
#[test]
fn a_realized_head_is_reused_when_the_tail_is_appended() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        for count in edit_points(total).into_iter().filter(|count| *count > 0) {
            let mut history = SessionHistory::new();
            history.realize(&head(&fixture.source, count));
            let plan = history.align(&fixture.source);

            let where_ = format!("{} @{count}", fixture.name);
            assert_eq!(plan.prefix_len, count, "{where_}");
            assert_eq!(plan.summary().reused, count, "{where_}");
            assert_eq!(plan.residue_len, 0, "{where_}");
            for statement in &plan.steps {
                let expected = if statement.index < count { Reuse } else { Run };
                assert_eq!(statement.action, expected, "{where_} #{}", statement.index);
            }
        }
    }
}

/// 셸에서 다음 줄을 치는 경우 — 딱 그 한 줄만 실행된다. 이 도구의 헤드라인이
/// 실제 코드에서 성립하는지를 보는 자리다.
#[test]
fn appending_one_statement_runs_only_that_statement() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        let grown = append_probe(&fixture.source);
        assert_eq!(grown.statements().len(), total + 1, "{}", fixture.name);

        let history = realized(&fixture.source);
        let plan = history.align(&grown);

        assert_eq!(plan.summary().run, 1, "{}: {}", fixture.name, actions(&plan));
        assert_eq!(plan.summary().reused, total, "{}", fixture.name);
        assert_eq!(plan.summary().first_run, Some(total), "{}", fixture.name);
        assert_eq!(plan.residue_len, 0, "{}", fixture.name);
    }
}

/// 중간에 한 줄을 끼워 넣으면, 그 아래는 위치가 밀려 더 이상 "그 자리의 실행"이
/// 아니다 — canonical이 같아도 전부 Run이다.
#[test]
fn inserting_a_statement_runs_everything_from_that_point() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        for index in edit_points(total) {
            let edited = insert_probe(&fixture.source, index);
            let history = realized(&fixture.source);
            let plan = history.align(&edited);

            let where_ = format!("{} @{index}", fixture.name);
            assert_eq!(plan.prefix_len, index, "{where_}");
            // 삽입 지점은 반드시 Run이다. 그 위가 오염되었으면 첫 Run은 더 위일 수 있다.
            assert!(
                plan.summary().first_run.expect("삽입 지점은 언제나 Run이다") <= index,
                "{where_}: {}",
                actions(&plan)
            );
            for statement in plan.steps.iter().filter(|s| s.index >= index) {
                assert_eq!(statement.action, Run, "{where_} #{}", statement.index);
            }
        }
    }
}

/// Reuse는 prefix 밖에서 절대 나오지 않는다.
#[test]
fn reuse_never_appears_beyond_the_prefix() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        for index in edit_points(total) {
            let edited = insert_probe(&fixture.source, index);
            let history = realized(&fixture.source);
            let plan = history.align(&edited);

            assert_eq!(plan.summary().reused + plan.summary().run, plan.summary().total);
            for statement in plan.steps.iter().filter(|step| step.action == Reuse) {
                assert!(statement.index < plan.prefix_len, "{}", fixture.name);
            }
        }
    }
}

/// 계획을 실행하고 realize하면 그 자리에서 수렴한다 — 편집을 몇 번 하든.
#[test]
fn the_edit_loop_converges_after_every_realize() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        let mut history = SessionHistory::new();
        history.realize(&fixture.source);
        assert!(history.align(&fixture.source).run_steps().next().is_none());

        for index in edit_points(total) {
            let edited = insert_probe(&fixture.source, index);
            history.realize(&edited);
            let plan = history.align(&edited);
            assert!(
                plan.run_steps().next().is_none(),
                "{} @{index}: 수렴하지 않았다 {}",
                fixture.name,
                actions(&plan)
            );
        }
    }
}

/// 앞뒤를 오가며 두 판본 사이를 왔다갔다 해도, realize할 때마다 수렴하고 갈라지는
/// 지점을 정확히 같은 자리로 잡는다. 세션이 시간이 지나며 썩지 않는다는 뜻이다.
#[test]
fn going_back_and_forth_between_two_versions_stays_sharp() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        if total < 3 {
            continue;
        }
        let split = total / 2;
        let first = &fixture.source;
        let second = insert_probe(first, split);

        let mut history = SessionHistory::new();
        for round in 0..3 {
            history.realize(first);
            let plan = history.align(first);
            assert!(
                plan.run_steps().next().is_none(),
                "{} round {round} A: {}",
                fixture.name,
                actions(&plan)
            );
            // 지금 실현된 것은 A다. B는 split 자리에서 갈라진다.
            assert_eq!(history.align(&second).prefix_len, split, "{}", fixture.name);

            history.realize(&second);
            let plan = history.align(&second);
            assert!(
                plan.run_steps().next().is_none(),
                "{} round {round} B: {}",
                fixture.name,
                actions(&plan)
            );
            assert_eq!(history.align(first).prefix_len, split, "{}", fixture.name);
        }

        assert_eq!(history.statement_count(), second.statements().len());
    }
}

/// 마지막 statement가 끊긴 세션. 끊긴 실행은 실현 열에 없으므로 그 자리는 반드시
/// 다시 돌고, 고쳐서 끝까지 돌리면 그 자리에서 수렴한다 — 끊긴 실행은 세션을
/// 영구히 못 쓰게 만들지 않는다.
#[test]
fn an_interrupted_last_statement_still_converges() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        if total < 2 {
            continue;
        }

        let mut history = SessionHistory::new();
        history.realize(&head(&fixture.source, total - 1));
        history.record_partial(&nth(&fixture.source, total - 1));

        let plan = history.align(&fixture.source);
        assert_eq!(plan.steps[total - 1].action, Run, "{}", fixture.name);
        assert_eq!(plan.residue_len, 1, "{}", fixture.name);

        history.realize(&fixture.source);
        let plan = history.align(&fixture.source);
        assert!(
            plan.run_steps().next().is_none(),
            "{}: 수렴하지 않았다 {}",
            fixture.name,
            actions(&plan)
        );
    }
}

/// 편집-실행을 반복해도 세션이 드는 실현 밖 실행은 유계다.
///
/// 한 줄을 고쳤다 되돌리는 사이클은 매번 같은 실행들을 실현 밖으로 민다. 그 실행들은
/// 오염 상계가 같으므로 뒤엣것이 앞엣것을 덮고, 세션이 드는 양은 몇 사이클 만에
/// 자리를 잡는다. 여기가 자라면 `align`이 사이클마다 느려진다 — 판정은 그대로인 채로.
#[test]
fn repeating_an_edit_loop_keeps_the_session_bounded() {
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        if total < 3 {
            continue;
        }
        let edited = replace_statement(&fixture.source, total / 2);

        let mut history = SessionHistory::new();
        history.realize(&fixture.source);
        for _ in 0..3 {
            history.realize(&edited);
            history.realize(&fixture.source);
        }
        let settled = history.residue_count();

        for _ in 0..30 {
            history.realize(&edited);
            history.realize(&fixture.source);
        }
        assert_eq!(history.residue_count(), settled, "{}", fixture.name);
        // 그리고 판정은 내내 수렴한 상태다.
        let plan = history.align(&fixture.source);
        assert!(
            plan.run_steps().next().is_none(),
            "{}: {}",
            fixture.name,
            actions(&plan)
        );
    }
}

#[test]
fn a_poisoned_session_runs_everything() {
    for fixture in corpus() {
        let mut history = realized(&fixture.source);
        history.poison();
        let plan = history.align(&fixture.source);
        assert_eq!(plan.summary().run, plan.summary().total, "{}", fixture.name);
    }
}

#[test]
fn downgrade_from_zero_turns_the_whole_plan_into_run() {
    for fixture in corpus() {
        let history = realized(&fixture.source);
        let mut plan = history.align(&fixture.source);
        assert_eq!(plan.summary().run, 0, "{}", fixture.name);

        plan.downgrade_from(0);
        assert_eq!(plan.summary().run, plan.summary().total, "{}", fixture.name);
        assert_eq!(plan.summary().reused, 0, "{}", fixture.name);
    }
}

#[test]
fn run_steps_is_the_run_subset_in_source_order() {
    for fixture in corpus() {
        let history = realized(&head(&fixture.source, 1));
        let plan = history.align(&fixture.source);

        let runs: Vec<usize> = plan.run_steps().map(|statement| statement.index).collect();
        assert_eq!(runs.len(), plan.summary().run, "{}", fixture.name);
        assert!(runs.windows(2).all(|pair| pair[0] < pair[1]), "{}", fixture.name);
        assert!(runs.iter().all(|index| plan.steps[*index].action == Run));
    }
}

/// 맨 아래를 고쳤을 때 실제로 얼마나 아끼는가. 판정 규칙이 보장하는 것만
/// 단언하고(갈라지는 지점은 정확히 맨 아래다), 실제 재사용 비율은 찍어 둔다 —
/// `cargo test --test corpus -- --nocapture reuse_report`로 본다.
#[test]
fn reuse_report_for_a_bottom_edit() {
    let mut report = String::from("\n소스별 재사용 (맨 아래 statement를 고쳤을 때)\n");
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        let bottom = *edit_points(total).last().expect("편집 지점이 하나는 있다");
        let edited = insert_probe(&fixture.source, bottom);
        let history = realized(&fixture.source);
        let plan = history.align(&edited);

        assert_eq!(plan.prefix_len, bottom, "{}", fixture.name);
        report.push_str(&format!(
            "  {:<34} {:>4}/{:<4} 재사용  ({:>3}%)\n",
            fixture.name,
            plan.summary().reused,
            plan.summary().total,
            100 * plan.summary().reused / plan.summary().total,
        ));
    }
    println!("{report}");
}
