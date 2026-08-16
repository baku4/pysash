//! Measured cell-level reuse baselines for in-place statement edits.

mod support;

use pysash::plan::Action;
use support::{cell_of_statement, cell_reuse, corpus, realized, replace_statement};

/// File and surviving-cell counts for middle and bottom edits.
const BASELINE: &[(&str, usize, usize, usize, usize)] = &[
    ("module_metrics.py", 2, 2, 4, 4),                  // 5 cells: middle 100%, bottom 100%
    ("module_plotting.py", 4, 4, 7, 7),                 // 8 cells: middle 100%, bottom 100%
    ("module_tools.py", 4, 4, 7, 7),                    // 8 cells: middle 100%, bottom 100%
    ("notebook_compare_tools.py", 9, 5, 18, 18),        // 19 cells: middle 55%, bottom 100%
    ("notebook_contig_length_qc.py", 4, 3, 16, 14),     // 17 cells: middle 75%, bottom 87%
    ("notebook_define_file_paths.py", 13, 5, 29, 28),   // 30 cells: middle 38%, bottom 96%
    ("notebook_gwas_manhattan.py", 13, 8, 31, 16),      // 32 cells: middle 61%, bottom 51%
    ("notebook_merge_lineage.py", 7, 4, 18, 18),        // 19 cells: middle 57%, bottom 100%
    ("notebook_simulate_reads.py", 20, 10, 38, 36),     // 39 cells: middle 50%, bottom 94%
    ("notebook_upload_assembly.py", 6, 4, 17, 17),      // 18 cells: middle 66%, bottom 100%
    ("notebook_variants_table.py", 5, 3, 18, 18),       // 19 cells: middle 60%, bottom 100%
    ("script_download_assemblies.py", 2, 0, 8, 4),      // 9 cells: middle 0%, bottom 50%
    ("script_pairwise_ani.py", 3, 1, 12, 8),            // 13 cells: middle 33%, bottom 66%
    ("script_run_skani.py", 1, 0, 4, 2),                // 5 cells: middle 0%, bottom 50%
    // Total: middle edit 53/93 (56%), bottom edit 197/227 (86%).
];

/// Cells eligible for reuse and cells actually reused by one edit.
type Edit = (usize, usize);

struct Measured {
    name: String,
    cells: usize,
    half: Edit,
    bottom: Edit,
}

/// Counts cells entirely above the statement at `index`.
fn cells_above(cells: &[usize], index: usize) -> usize {
    let edited = cells[index];
    let mut above: Vec<usize> = cells[..index]
        .iter()
        .copied()
        .filter(|cell| *cell != edited)
        .collect();
    above.sort_unstable();
    above.dedup();
    above.len()
}

/// Measures eligible and surviving cells after replacing the statement at `index`.
fn measure(fixture: &support::Fixture, index: usize) -> Edit {
    let edited = replace_statement(&fixture.source, index);
    let history = realized(&fixture.source);
    let plan = history.align(&edited);

    assert_eq!(
        plan.prefix_len, index,
        "{} @{index}: incorrect split point",
        fixture.name
    );
    for statement in plan.steps.iter().filter(|s| s.index >= index) {
        assert_eq!(
            statement.action,
            Action::Run,
            "{} @{index}: #{} is reused below the edit point",
            fixture.name,
            statement.index
        );
    }

    let cells = cell_of_statement(&edited);
    let (reused, _) = cell_reuse(&plan, &cells);
    (cells_above(&cells, index), reused)
}

#[test]
fn cell_level_reuse_matches_the_baseline() {
    let measured: Vec<Measured> = corpus()
        .iter()
        .map(|fixture| {
            let total = fixture.source.statements().len();
            // Markdown-only cells contain no statements and are not execution units.
            let mut occupied = cell_of_statement(&fixture.source);
            occupied.sort_unstable();
            occupied.dedup();
            Measured {
                name: fixture.name.clone(),
                cells: occupied.len(),
                half: measure(fixture, total / 2),
                bottom: measure(fixture, total - 1),
            }
        })
        .collect();

    let actual: Vec<(&str, usize, usize, usize, usize)> = measured
        .iter()
        .map(|m| (m.name.as_str(), m.half.0, m.half.1, m.bottom.0, m.bottom.1))
        .collect();

    assert_eq!(
        actual,
        BASELINE.to_vec(),
        "\nreuse baseline changed\n{}",
        table(&measured)
    );
}

/// Formats measured values for direct replacement of the baseline table.
fn table(measured: &[Measured]) -> String {
    let percent = |edit: Edit| if edit.0 == 0 { 100 } else { 100 * edit.1 / edit.0 };
    let mut out = String::from("Measured values (ready to paste):\n");
    let (mut above_half, mut half, mut above_bottom, mut bottom) = (0, 0, 0, 0);
    for m in measured {
        let entry = format!(
            "    (\"{}\", {}, {}, {}, {}),",
            m.name, m.half.0, m.half.1, m.bottom.0, m.bottom.1
        );
        out.push_str(&format!(
            "{entry:<56}// {} cells: middle {}%, bottom {}%\n",
            m.cells,
            percent(m.half),
            percent(m.bottom),
        ));
        above_half += m.half.0;
        half += m.half.1;
        above_bottom += m.bottom.0;
        bottom += m.bottom.1;
    }
    out.push_str(&format!(
        "\n    Total: middle edit {half}/{above_half} ({}%), bottom edit {bottom}/{above_bottom} ({}%)\n",
        100 * half / above_half,
        100 * bottom / above_bottom,
    ));
    out
}
