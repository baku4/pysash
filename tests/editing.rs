//! Expected reuse decisions for representative source-edit workflows.

mod support;

use pysash::SessionHistory;
use pysash::plan::{DecisionReason, Effect, SessionDiagnostic, StatementDiagnostic};
use support::{actions, explain, has_diagnostic, head, nth, realized, reasons, step};

const BASE: &str = "01_base.py";

// ---------------------------------------------------------------------------
// Contig QC: edit a ten-statement source in both directions.
// ---------------------------------------------------------------------------

/// Appending to a clean prefix runs only the two new statements.
#[test]
fn appending_below_reuses_everything_above() {
    let mut history = realized(&step("contig_qc", BASE));
    let grown = step("contig_qc", "02_grown.py");

    let plan = history.align(&grown);
    assert_eq!(actions(&plan), "..........XX", "{}", explain(&plan, &grown));
    assert_eq!(plan.prefix_len, 10);
    assert_eq!(plan.residue_len, 0);
    assert_eq!(plan.summary().first_run, Some(10));
    assert_eq!(plan.steps[10].reason, DecisionReason::NoMatchingExecution);
    assert!(plan.diagnostics.is_empty());
    // The write effect is exposed for caller policy.
    assert_eq!(plan.steps[10].effect, Effect::ExternalWrite);

    history.realize(&grown);
    assert!(history.align(&grown).run_steps().next().is_none());
}

/// Editing only the final output statement preserves the first nine statements.
#[test]
fn editing_only_the_last_line_reuses_everything_above() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "03_edit_last_line.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".........X", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 9);
    assert_eq!(plan.residue_len, 1);
    assert_eq!(plan.steps[9].reason, DecisionReason::StatementChanged);
    // `residue_len` records the displaced execution without a diagnostic.
    assert!(plan.diagnostics.is_empty());
}

/// Editing the threshold reruns only prefix producers possibly mutated by the old suffix.
#[test]
fn editing_a_constant_near_the_top_reruns_from_there() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "04_edit_threshold.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".X..XXXXXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 5);
    assert_eq!(plan.residue_len, 5);

    let reasons = reasons(&plan);
    assert_eq!(reasons[0], DecisionReason::ReusableExecution);
    assert_eq!(
        reasons[1],
        DecisionReason::DependencyChanged { name: "pd".into() }
    );
    assert_eq!(
        reasons[4],
        DecisionReason::DependencyChanged {
            name: "CONTIG_LEN_FILE".into()
        }
    );
    assert_eq!(reasons[5], DecisionReason::StatementChanged);
}

/// Insertion invalidates positional witnesses below it while preserving safe prefix entries.
#[test]
fn inserting_a_cell_shifts_everything_below_into_run() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "05_insert_display_option.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".X..X.XXXXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 6);
    assert_eq!(plan.residue_len, 4);

    let reasons = reasons(&plan);
    // Nothing disturbed the threshold binding.
    assert_eq!(reasons[5], DecisionReason::ReusableExecution);
    assert_eq!(reasons[6], DecisionReason::StatementChanged);
    // The old statement at index 7 is not a witness for its new position.
    assert_eq!(
        reasons[7],
        DecisionReason::DependencyChanged { name: "pd".into() }
    );
}

/// Deleting a lower statement preserves the undisturbed first eight statements.
#[test]
fn deleting_a_cell_near_the_bottom_keeps_the_head() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "06_drop_a_line.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "........X", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 8);
    assert_eq!(plan.residue_len, 2);
    assert_eq!(plan.steps[8].reason, DecisionReason::StatementChanged);
}

/// Reordering the first two statements yields an empty positional prefix.
#[test]
fn reordering_two_imports_destroys_the_prefix() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "07_reordered_imports.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "XXXXXXXXXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 0);
    assert_eq!(plan.summary().reused, 0);

    let reasons = reasons(&plan);
    assert_eq!(reasons[0], DecisionReason::StatementChanged);
    // A matching statement in another position has no valid contextual witness.
    assert_eq!(reasons[1], DecisionReason::NoMatchingExecution);
    assert_eq!(
        reasons[2],
        DecisionReason::DependencyChanged {
            name: "Path".into()
        }
    );
}

/// Formatting-only edits preserve every canonical statement.
#[test]
fn reformatting_costs_nothing() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "08_reformatted.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "..........", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 10);
    assert_eq!(plan.residue_len, 0);
}

/// Alternating between two versions converges after each realization.
#[test]
fn going_back_and_forth_converges_at_each_realize() {
    let base = step("contig_qc", BASE);
    let edited = step("contig_qc", "04_edit_threshold.py");

    let mut history = SessionHistory::new();
    history.realize(&base);
    assert!(history.align(&base).run_steps().next().is_none());

    // Move down by editing the threshold.
    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".X..XXXXXX", "{}", explain(&plan, &edited));
    history.realize(&edited);
    assert!(history.align(&edited).run_steps().next().is_none());
    assert_eq!(history.residue_count(), 5);

    // Move back to the base version with the symmetric verdict.
    let plan = history.align(&base);
    assert_eq!(actions(&plan), ".X..XXXXXX", "{}", explain(&plan, &base));
    assert_eq!(plan.prefix_len, 5);
    assert_eq!(plan.residue_len, 10);

    history.realize(&base);
    assert!(history.align(&base).run_steps().next().is_none());
    assert_eq!(history.statement_count(), 10);
    // The newest five disturbance bounds subsume the ten displaced executions.
    assert_eq!(history.residue_count(), 5);

    // Repeated alternation does not increase retained residue.
    for _ in 0..50 {
        history.realize(&edited);
        history.realize(&base);
    }
    assert!(history.align(&base).run_steps().next().is_none());
    assert_eq!(history.residue_count(), 5);
}

/// Caller policy may rerun an external read even when the execution is reusable.
#[test]
fn the_caller_can_downgrade_from_the_external_read() {
    let base = step("contig_qc", BASE);
    let history = realized(&base);
    let mut plan = history.align(&base);
    assert_eq!(actions(&plan), "..........");

    let read = plan
        .steps
        .iter()
        .find(|statement| statement.effect == Effect::ExternalRead)
        .map(|statement| statement.index)
        .expect("fixture contains pd.read_csv");
    assert_eq!(read, 6);

    plan.downgrade_from(read);
    assert_eq!(actions(&plan), "......XXXX");
    assert_eq!(plan.summary().reused, 6);
    assert_eq!(plan.summary().first_run, Some(6));
    // The original reusable reason survives the caller downgrade.
    assert_eq!(plan.steps[6].reason, DecisionReason::ReusableExecution);
}

/// Poisoning an unknowable session forces every subsequent statement to run.
#[test]
fn a_poisoned_session_runs_everything_for_good() {
    let base = step("contig_qc", BASE);
    let mut history = realized(&base);
    history.poison();

    let plan = history.align(&base);
    assert_eq!(actions(&plan), "XXXXXXXXXX");
    assert!(reasons(&plan)
        .iter()
        .all(|reason| *reason == DecisionReason::NoMatchingExecution));
}

/// Recording a failed eighth cell preserves the completed first seven cells.
#[test]
fn an_interrupted_cell_costs_only_what_it_could_have_touched() {
    let base = step("contig_qc", BASE);
    let mut history = SessionHistory::new();
    history.realize(&head(&base, 7));
    history.record_partial(&nth(&base, 7));

    let plan = history.align(&base);
    assert_eq!(actions(&plan), ".......XXX", "{}", explain(&plan, &base));
    assert_eq!(plan.prefix_len, 7);
    assert_eq!(plan.residue_len, 1);
    assert_eq!(history.residue_count(), 1);
    // The interrupted cell is residue rather than a positional witness.
    assert_eq!(plan.steps[7].reason, DecisionReason::NoMatchingExecution);

    // A successful rerun receives a later sequence and converges immediately.
    history.realize(&base);
    assert!(history.align(&base).run_steps().next().is_none());
}

/// Poisoning the same failure also invalidates the completed prefix.
#[test]
fn poisoning_the_same_failure_throws_away_the_whole_session() {
    let base = step("contig_qc", BASE);
    let mut history = SessionHistory::new();
    history.realize(&head(&base, 7));
    history.poison();

    assert_eq!(actions(&history.align(&base)), "XXXXXXXXXX");
}

/// A self-contained source has no unresolved-reference diagnostics.
#[test]
fn a_self_contained_source_has_no_statement_diagnostics() {
    let base = step("contig_qc", BASE);
    let plan = SessionHistory::new().align(&base);
    assert!(plan
        .steps
        .iter()
        .all(|statement| statement.diagnostics.is_empty()));
}

// ---------------------------------------------------------------------------
// Notebook prologue: `from … import *` inside and outside the prefix.
// ---------------------------------------------------------------------------

/// An opaque import inside the common prefix remains reusable.
#[test]
fn a_star_import_inside_the_prefix_is_harmless() {
    let history = realized(&step("notebook_prologue", BASE));
    let edited = step("notebook_prologue", "02_edit_config_path.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".....XXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 5);
    assert_eq!(plan.residue_len, 3);

    // Opaque classification does not block a matching prefix witness.
    assert_eq!(plan.steps[4].effect, Effect::Opaque);
    assert!(!has_diagnostic(&plan, |d| matches!(
        d,
        SessionDiagnostic::OpaqueResidue { .. }
    )));

    // `parse_config` is unresolved because this source never binds it.
    assert!(plan.steps[6].diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        StatementDiagnostic::UnresolvedReference { name } if &**name == "parse_config"
    )));
}

/// Moving an opaque import outside the prefix forces every statement to run.
#[test]
fn a_star_import_pushed_out_of_the_prefix_runs_everything() {
    let history = realized(&step("notebook_prologue", BASE));
    let edited = step("notebook_prologue", "03_edit_sys_path.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "XXXXXXXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 3);
    assert_eq!(plan.residue_len, 5);
    assert!(has_diagnostic(&plan, |d| matches!(
        d,
        SessionDiagnostic::OpaqueResidue { .. }
    )));

    let reasons = reasons(&plan);
    // `sys.path.append(...)` may mutate `sys`, disturbing its import first.
    assert_eq!(
        reasons[0],
        DecisionReason::DependencyChanged { name: "sys".into() }
    );
    // The opaque residue supplies no narrower name-based reason.
    assert_eq!(reasons[1], DecisionReason::NoMatchingExecution);
    assert_eq!(reasons[2], DecisionReason::NoMatchingExecution);
    assert_eq!(reasons[3], DecisionReason::StatementChanged);
}

// ---------------------------------------------------------------------------
// Two representative notebook shapes.
// ---------------------------------------------------------------------------

/// Editing `pd.concat` reruns the list producer that the old call may have mutated.
#[test]
fn an_accumulating_list_reruns_its_producer() {
    let history = realized(&step("merge_results", BASE));
    let edited = step("merge_results", "02_reset_index.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "X.XXXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 4);
    assert_eq!(plan.residue_len, 2);

    let reasons = reasons(&plan);
    assert_eq!(
        reasons[0],
        DecisionReason::DependencyChanged { name: "pd".into() }
    );
    // The file list remains undisturbed.
    assert_eq!(reasons[1], DecisionReason::ReusableExecution);
    assert_eq!(
        reasons[2],
        DecisionReason::DependencyChanged {
            name: "df_list".into()
        }
    );
    assert_eq!(reasons[4], DecisionReason::StatementChanged);
}

/// Editing a helper preserves unrelated setup but reruns a possibly mutated dataframe producer.
#[test]
fn editing_a_helper_function_leaves_run_gaps() {
    let history = realized(&step("gwas_labeling", BASE));
    let edited = step("gwas_labeling", "02_edit_label_fn.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "..X.XXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 4);
    assert_eq!(plan.residue_len, 3);

    let reasons = reasons(&plan);
    assert_eq!(reasons[0], DecisionReason::ReusableExecution);
    assert_eq!(
        reasons[2],
        DecisionReason::DependencyChanged {
            name: "gwas_df".into()
        }
    );
    assert_eq!(reasons[3], DecisionReason::ReusableExecution);
    assert_eq!(reasons[4], DecisionReason::StatementChanged);
    assert_eq!(plan.steps[2].effect, Effect::ExternalRead);
}
