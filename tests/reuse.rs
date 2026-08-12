//! 재사용률 기준선 — 이 도구가 실제로 아끼는 양.
//!
//! 판정은 statement 하나하나에 내려지지만, 셸은 **셀 단위로 실행한다**. 셀 안에
//! Run이 하나라도 있으면 그 셀은 통째로 돌아간다. 그래서 여기서 세는 단위는
//! statement가 아니라 셀이다 — 사용자가 실제로 기다리는 시간이 그쪽에 비례한다.
//!
//! 편집은 **제자리 수정**이다. statement 하나를 다른 한 줄로 갈아 끼운다 — 줄을
//! 끼워 넣거나 지우지 않으므로 셀 구조가 그대로고, 셸에서 한 줄 고치는 그 편집과
//! 같은 모양이다.
//!
//! 재는 것은 **상한 대비 생존**이다. 편집 지점 아래는 판정상 무조건 다시 도니
//! 전체 셀로 나누면 "가운데를 고쳤다"와 "맨 아래를 고쳤다"를 비교할 수 없다.
//! 분모는 *편집 지점보다 온전히 위에 있는 셀* — 이론상 전부 살아남을 수 있는
//! 셀들이다. 100%면 이 도구가 낼 수 있는 최선을 낸 것이다.
//!
//! 아래 표의 숫자는 판정 규칙에서 유도한 것이 아니라 **측정한 기준선**이다. 정밀도가
//! 나빠지면 실패하고, 좋아져도 실패한다 — 좋아졌을 때 표를 고치면 그 diff 자체가
//! 개선의 증거가 된다. 실패하면 붙여 넣을 표를 그대로 찍어 준다.

mod support;

use pysash::plan::Action;
use support::{cell_of_statement, cell_reuse, corpus, realized, replace_statement};

/// (파일, 가운데 수정: 위쪽 셀, 그중 생존, 맨 아래 수정: 위쪽 셀, 그중 생존)
const BASELINE: &[(&str, usize, usize, usize, usize)] = &[
    ("module_metrics.py", 2, 2, 4, 4),                  // 5셀 — 가운데 100%, 맨 아래 100%
    ("module_plotting.py", 4, 4, 7, 7),                 // 8셀 — 가운데 100%, 맨 아래 100%
    ("module_tools.py", 4, 4, 7, 7),                    // 8셀 — 가운데 100%, 맨 아래 100%
    ("notebook_compare_tools.py", 9, 5, 18, 18),        // 19셀 — 가운데 55%, 맨 아래 100%
    ("notebook_contig_length_qc.py", 4, 3, 16, 14),     // 17셀 — 가운데 75%, 맨 아래 87%
    ("notebook_define_file_paths.py", 13, 5, 29, 28),   // 30셀 — 가운데 38%, 맨 아래 96%
    ("notebook_gwas_manhattan.py", 13, 8, 31, 16),      // 32셀 — 가운데 61%, 맨 아래 51%
    ("notebook_merge_lineage.py", 7, 4, 18, 18),        // 19셀 — 가운데 57%, 맨 아래 100%
    ("notebook_simulate_reads.py", 20, 10, 38, 36),     // 39셀 — 가운데 50%, 맨 아래 94%
    ("notebook_upload_assembly.py", 6, 4, 17, 17),      // 18셀 — 가운데 66%, 맨 아래 100%
    ("notebook_variants_table.py", 5, 3, 18, 18),       // 19셀 — 가운데 60%, 맨 아래 100%
    ("script_download_assemblies.py", 2, 0, 8, 4),      // 9셀 — 가운데 0%, 맨 아래 50%
    ("script_pairwise_ani.py", 3, 1, 12, 8),            // 13셀 — 가운데 33%, 맨 아래 66%
    ("script_run_skani.py", 1, 0, 4, 2),                // 5셀 — 가운데 0%, 맨 아래 50%
    // 합계 — 가운데 수정 53/93 (56%), 맨 아래 수정 197/227 (86%)
];

/// 편집 한 번에 대한 측정 — (살아남을 수 있었던 셀, 실제로 살아남은 셀).
type Edit = (usize, usize);

struct Measured {
    name: String,
    cells: usize,
    half: Edit,
    bottom: Edit,
}

/// `index`보다 **온전히 위**에 있는 셀 수. 편집 지점이 걸친 셀은 빠진다.
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

/// `index` 자리를 제자리 수정했을 때의 (상한, 생존).
///
/// 재는 김에 판정의 뼈대도 확인한다 — 갈라지는 지점이 정확히 그 자리이고, 그
/// 아래는 하나도 빠짐없이 Run이다. 제자리 수정은 아래 statement의 index도
/// canonical도 그대로 두므로, 위치로 고정한다는 것이 바로 여기서 시험된다.
fn measure(fixture: &support::Fixture, index: usize) -> Edit {
    let edited = replace_statement(&fixture.source, index);
    let history = realized(&fixture.source);
    let plan = history.align(&edited);

    assert_eq!(
        plan.summary.prefix_len, index,
        "{} @{index}: 갈라지는 지점이 어긋났다",
        fixture.name
    );
    for statement in plan.plans.iter().filter(|s| s.index >= index) {
        assert_eq!(
            statement.action,
            Action::Run,
            "{} @{index}: #{}은 편집 지점 아래인데 Reuse다",
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
            // markdown 셀에는 statement가 없다 — 실행 단위로 세지 않는다.
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
        "\n기준선이 어긋났다.\n{}",
        table(&measured)
    );
}

/// 기준선 표를 그대로 붙여 넣을 수 있게 찍는다.
fn table(measured: &[Measured]) -> String {
    let percent = |edit: Edit| if edit.0 == 0 { 100 } else { 100 * edit.1 / edit.0 };
    let mut out = String::from("측정값 (붙여 넣으면 된다):\n");
    let (mut above_half, mut half, mut above_bottom, mut bottom) = (0, 0, 0, 0);
    for m in measured {
        let entry = format!(
            "    (\"{}\", {}, {}, {}, {}),",
            m.name, m.half.0, m.half.1, m.bottom.0, m.bottom.1
        );
        out.push_str(&format!(
            "{entry:<56}// {}셀 — 가운데 {}%, 맨 아래 {}%\n",
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
        "\n    합계 — 가운데 수정 {half}/{above_half} ({}%), 맨 아래 수정 {bottom}/{above_bottom} ({}%)\n",
        100 * half / above_half,
        100 * bottom / above_bottom,
    ));
    out
}
