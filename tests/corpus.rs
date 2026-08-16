//! General alignment invariants exercised against extracted research-code fixtures.

mod support;

use pysash::SessionHistory;
use pysash::plan::Action;
use pysash::source::PythonSource;
use support::{actions, append_probe, corpus, head, insert_probe, nth, realized, replace_statement};
use Action::{Reuse, Run};

/// Chooses front, early, middle, and final edit positions when distinct.
fn edit_points(total: usize) -> Vec<usize> {
    assert!(total > 0, "cannot choose an edit point in an empty fixture");
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
            "{}: fixture contains no statements",
            fixture.name
        );
        // Statement ranges are measured against the unchanged input bytes.
        assert_eq!(fixture.source.raw(), fixture.text.as_bytes(), "{}", fixture.name);
    }
}

/// Every statement slice reparses to the same canonical statement.
#[test]
fn every_statement_slices_back_to_the_same_statement() {
    for fixture in corpus() {
        for statement in fixture.source.statements() {
            let text = std::str::from_utf8(fixture.source.slice(statement.range))
                .expect("statement boundaries are UTF-8 boundaries");
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

/// Source pushed directly into a clean session is fully reusable.
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

/// Alignment is deterministic and does not mutate the session.
#[test]
fn align_does_not_change_the_session() {
    for fixture in corpus() {
        let history = realized(&fixture.source);
        let before = history.clone();

        let first = history.align(&fixture.source);
        let second = history.align(&fixture.source);

        assert_eq!(first, second, "{}", fixture.name);
        assert!(history == before, "{}: align mutated the session", fixture.name);
    }
}

/// A clean session prefix is entirely reusable.
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

/// Appending one statement runs only that statement.
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

/// Insertion invalidates every positional witness below the inserted statement.
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
            // Disturbance may move the first `Run` above the insertion point.
            assert!(
                plan.summary().first_run.expect("the inserted statement must run") <= index,
                "{where_}: {}",
                actions(&plan)
            );
            for statement in plan.steps.iter().filter(|s| s.index >= index) {
                assert_eq!(statement.action, Run, "{where_} #{}", statement.index);
            }
        }
    }
}

/// Reuse never appears outside the common prefix.
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

/// Realizing a plan converges immediately after every edit.
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
                "{} @{index}: did not converge {}",
                fixture.name,
                actions(&plan)
            );
        }
    }
}

/// Alternating between two versions preserves their split point and convergence.
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
            // Version A is realized, so version B still diverges at `split`.
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

/// A partial final statement must rerun and converges after successful completion.
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
            "{}: did not converge {}",
            fixture.name,
            actions(&plan)
        );
    }
}

/// Repeated edit-and-realize cycles retain a bounded amount of residue.
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
        // The realized source remains converged throughout.
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

/// Reports observed reuse for a bottom edit while asserting the exact split point.
#[test]
fn reuse_report_for_a_bottom_edit() {
    let mut report = String::from("\nReuse by source after editing the final statement\n");
    for fixture in corpus() {
        let total = fixture.source.statements().len();
        let bottom = *edit_points(total).last().expect("at least one edit point exists");
        let edited = insert_probe(&fixture.source, bottom);
        let history = realized(&fixture.source);
        let plan = history.align(&edited);

        assert_eq!(plan.prefix_len, bottom, "{}", fixture.name);
        report.push_str(&format!(
            "  {:<34} {:>4}/{:<4} reused  ({:>3}%)\n",
            fixture.name,
            plan.summary().reused,
            plan.summary().total,
            100 * plan.summary().reused / plan.summary().total,
        ));
    }
    println!("{report}");
}
