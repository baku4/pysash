//! 셸에서 소스를 위아래로 오가며 고칠 때, 무엇이 재사용되고 무엇이 다시 도는가.
//!
//! fixture는 `tests/fixtures/sessions/`에 있고 **셀 하나에 statement 하나**다 —
//! 아래 판정 문자열의 자리와 fixture의 셀 번호가 그대로 맞는다. 판정 문자열은
//! `.`이 Reuse, `X`가 Run이다.
//!
//! 기대값은 구현을 돌려 보고 받아 적은 것이 아니라 판정 규칙에서 손으로 유도했다 —
//! 공통 prefix가 어디까지인지, 실현 밖으로 밀려난 실행이 무엇을 오염시켰는지를
//! statement마다 따라간 결과다. 각 테스트의 주석이 그 유도이고, 구현이 여기서
//! 어긋나면 둘 중 하나가 틀린 것이다. 어느 쪽인지는 주석을 읽어서 가린다.

mod support;

use pysash::SessionHistory;
use pysash::plan::{DecisionReason, Effect, SessionDiagnostic, StatementDiagnostic};
use support::{actions, explain, has_diagnostic, realized, reasons, step};

const BASE: &str = "01_base.py";

// ---------------------------------------------------------------------------
// contig QC — 열 개짜리 소스를 위아래로 오가며 고친다.
// ---------------------------------------------------------------------------

/// 아래로 이어 붙이기. 세션이 소스의 순수 prefix이므로 밀려나는 실행이 없고,
/// 오염될 것도 없다 — 새로 친 두 줄만 돈다.
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
    // 밀려난 실행이 없으니 plan 전체에 붙일 주석도 없다.
    assert!(plan.diagnostics.is_empty());
    // `kept_df.to_csv(...)`는 외부 세계를 바꾼다 — 호출자가 후처리할 수 있게 분류된다.
    assert_eq!(plan.steps[10].effect, Effect::ExternalWrite);

    history.realize(&grown);
    assert!(history.align(&grown).run_steps().next().is_none());
}

/// 맨 아래 한 줄만 고친 경우. 밀려나는 실행은 옛 `print` 하나뿐인데, `print`는
/// 순수 호출 화이트리스트에 있어 이름을 바인딩하지도 인자를 in-place로 바꾸지도
/// 않는다. 오염 집합이 비어 위의 아홉 개가 전부 재사용된다.
#[test]
fn editing_only_the_last_line_reuses_everything_above() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "03_edit_last_line.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".........X", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 9);
    assert_eq!(plan.residue_len, 1);
    assert_eq!(plan.steps[9].reason, DecisionReason::StatementChanged);
    // 밀려난 실행이 있다는 사실은 residue_len이 말한다. 그것 말고 붙일 주석은 없다.
    assert!(plan.diagnostics.is_empty());
}

/// 위쪽 임계값을 고친 경우. 편집 지점(5) 아래는 당연히 전부 Run이고, 그 **위**가
/// 이 케이스의 요점이다.
///
/// 밀려난 `len_df = pd.read_csv(CONTIG_LEN_FILE)`는 정적으로는 receiver `pd`와
/// 인자 `CONTIG_LEN_FILE`을 in-place로 바꿨을 수 있는 실행이다 — 순수 화이트리스트
/// 밖의 메서드 호출은 receiver와 인자 전부를 mutation 후보로 잡는다. 그래서 그
/// 둘을 만든 statement 1과 4가 함께 다시 돈다. `Path` import와 경로 상수 두 개는
/// 아무도 건드리지 않았으므로 그대로다.
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

/// 중간에 셀 하나를 끼워 넣은 경우. 삽입 지점 아래는 위치가 한 칸씩 밀려 더 이상
/// "그 자리의 실행"이 아니다 — canonical이 같아도 전부 Run이다. 위쪽은 앞의
/// 케이스와 같은 이유로 1과 4만 다시 돈다. Run이 연속 구간이 아니라는 점을 본다.
#[test]
fn inserting_a_cell_shifts_everything_below_into_run() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "05_insert_display_option.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".X..X.XXXXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 6);
    assert_eq!(plan.residue_len, 4);

    let reasons = reasons(&plan);
    // 임계값(5)은 아무도 건드리지 않았다.
    assert_eq!(reasons[5], DecisionReason::ReusableExecution);
    assert_eq!(reasons[6], DecisionReason::StatementChanged);
    // 7은 세션이 실행한 적 있는 문장이지만, 그 자리의 실행이 아니다.
    assert_eq!(
        reasons[7],
        DecisionReason::DependencyChanged { name: "pd".into() }
    );
}

/// 아래쪽 셀 하나를 지운 경우. 밀려나는 것은 지워진 대입과 옛 `print`뿐이고
/// 둘 다 위쪽이 만든 이름을 건드리지 않는다 — 앞의 여덟 개가 그대로다.
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

/// 맨 위 두 줄의 순서만 바꾼 경우. 내용은 그대로지만 index 0의 statement가 달라져
/// 공통 prefix가 0이 된다. 세션은 linear한 기록이고 prefix는 위치로 고정된다 —
/// jupyter처럼 순서를 오가는 모델은 이 도구가 표현하지 않는다.
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
    // 세션이 실행한 적 있는 문장이지만 문맥이 다르다. 읽는 이름이 없으면 근거로
    // 댈 이름도 없다.
    assert_eq!(reasons[1], DecisionReason::NoMatchingExecution);
    assert_eq!(
        reasons[2],
        DecisionReason::DependencyChanged {
            name: "Path".into()
        }
    );
}

/// 주석, 따옴표 종류, `1000` → `1_000`, 잉여 괄호, 줄바꿈 — 전부 정규화가 흡수하는
/// 것들이라 AST가 같다. 한 줄도 다시 돌지 않는다.
#[test]
fn reformatting_costs_nothing() {
    let history = realized(&step("contig_qc", BASE));
    let edited = step("contig_qc", "08_reformatted.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), "..........", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 10);
    assert_eq!(plan.residue_len, 0);
}

/// **핵심 시나리오** — 아래로 내려가 고치고, 다시 위로 올라가 되돌린다.
///
/// 되돌아간 판정이 내려갈 때와 정확히 같다. 오염은 시간을 거슬러 일어나지
/// 않으므로, realize가 다시 실행된 자리에 새 순번을 달아 주면 이미 지나간 오염이
/// 그 자리를 영원히 Run으로 만들지 못한다. 몇 번을 오가도 realize할 때마다
/// 그 자리에서 수렴한다.
#[test]
fn going_back_and_forth_converges_at_each_realize() {
    let base = step("contig_qc", BASE);
    let edited = step("contig_qc", "04_edit_threshold.py");

    let mut history = SessionHistory::new();
    history.realize(&base);
    assert!(history.align(&base).run_steps().next().is_none());

    // 내려가서 임계값을 고친다.
    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".X..XXXXXX", "{}", explain(&plan, &edited));
    history.realize(&edited);
    assert!(history.align(&edited).run_steps().next().is_none());
    assert_eq!(history.residue_count(), 5);

    // 다시 올라가서 되돌린다 — 판정이 대칭이다.
    let plan = history.align(&base);
    assert_eq!(actions(&plan), ".X..XXXXXX", "{}", explain(&plan, &base));
    assert_eq!(plan.prefix_len, 5);
    assert_eq!(plan.residue_len, 10);

    history.realize(&base);
    assert!(history.align(&base).run_steps().next().is_none());
    assert_eq!(history.statement_count(), 10);
    assert_eq!(history.residue_count(), 10);
}

/// 판정 자체는 재사용 가능이라도, 외부 파일이 바뀌었을 수 있으니 읽는 지점부터
/// 다시 돌겠다는 것은 호출자의 정책이다. 라이브러리는 `Effect`만 알려 준다.
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
        .expect("pd.read_csv가 있다");
    assert_eq!(read, 6);

    plan.downgrade_from(read);
    assert_eq!(actions(&plan), "......XXXX");
    assert_eq!(plan.summary().reused, 6);
    assert_eq!(plan.summary().first_run, Some(6));
    // 판정의 기록은 남는다 — 재사용 가능했지만 호출자가 내렸다.
    assert_eq!(plan.steps[6].reason, DecisionReason::ReusableExecution);
}

/// 실행이 중간에 실패해 세션을 더는 믿을 수 없으면 이후는 전부 Run이다.
#[test]
fn a_failed_run_poisons_the_session_for_good() {
    let base = step("contig_qc", BASE);
    let mut history = realized(&base);
    history.poison();

    let plan = history.align(&base);
    assert_eq!(actions(&plan), "XXXXXXXXXX");
    assert!(reasons(&plan)
        .iter()
        .all(|reason| *reason == DecisionReason::NoMatchingExecution));
}

/// 자기 안에서 전부 바인딩되는 소스에는 statement 주석이 붙지 않는다.
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
// notebook 서두 — `from … import *`가 prefix 안일 때와 밖일 때.
// ---------------------------------------------------------------------------

/// `import *`(4)보다 아래를 고쳤다. opaque한 실행이 여전히 실현 열 안에 있으므로
/// 오염 집합에 들어가지 않는다 — 양쪽이 똑같이 실행한 것이기 때문이다.
/// 서두 다섯 줄이 그대로 재사용된다.
#[test]
fn a_star_import_inside_the_prefix_is_harmless() {
    let history = realized(&step("notebook_prologue", BASE));
    let edited = step("notebook_prologue", "02_edit_config_path.py");

    let plan = history.align(&edited);
    assert_eq!(actions(&plan), ".....XXX", "{}", explain(&plan, &edited));
    assert_eq!(plan.prefix_len, 5);
    assert_eq!(plan.residue_len, 3);

    // opaque로 분류되어 있지만 prefix 안이라 재사용된다.
    assert_eq!(plan.steps[4].effect, Effect::Opaque);
    assert!(!has_diagnostic(&plan, |d| matches!(
        d,
        SessionDiagnostic::OpaqueResidue { .. }
    )));

    // `parse_config`는 이 소스 어디에서도 바인딩되지 않는다 — 세션에서만 도는
    // 조각이라는 신호다.
    assert!(plan.steps[6].diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        StatementDiagnostic::UnresolvedReference { name } if &**name == "parse_config"
    )));
}

/// `import *`(4)보다 위를 고쳤다. 이제 opaque한 실행이 실현 열 밖으로 밀려나
/// 오염 집합이 전체가 된다 — 무엇이 망가졌는지 알 수 없으므로 전부 Run이다.
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
    // `sys.path.append(...)`는 receiver `sys`를 바꿨을 수 있다 — import가 먼저 걸린다.
    assert_eq!(
        reasons[0],
        DecisionReason::DependencyChanged { name: "sys".into() }
    );
    // 그 뒤는 opaque에 걸린다. 무엇이 오염됐는지 댈 이름이 없다.
    assert_eq!(reasons[1], DecisionReason::NoMatchingExecution);
    assert_eq!(reasons[2], DecisionReason::NoMatchingExecution);
    assert_eq!(reasons[3], DecisionReason::StatementChanged);
}

// ---------------------------------------------------------------------------
// 실제 notebook의 두 가지 흔한 모양.
// ---------------------------------------------------------------------------

/// `df_list = []` → 루프에서 `append` → `pd.concat`. 마지막에서 두 번째 줄만
/// 고쳤는데 `df_list = []`가 다시 돈다 — 밀려난 `pd.concat(df_list, axis=0)`이
/// 인자 `df_list`를 in-place로 바꿨을 수 있기 때문이다. 세션의 `df_list`에는 이미
/// 두 개가 들어 있으니, 빈 리스트를 다시 만들지 않으면 값이 어긋난다.
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
    // 파일 목록은 아무도 건드리지 않았다.
    assert_eq!(reasons[1], DecisionReason::ReusableExecution);
    assert_eq!(
        reasons[2],
        DecisionReason::DependencyChanged {
            name: "df_list".into()
        }
    );
    assert_eq!(reasons[4], DecisionReason::StatementChanged);
}

/// 아래쪽 헬퍼 함수 본문만 고쳤다. `import pandas as pd`, 파일 경로 상수,
/// `DRUG_LIST`는 그대로 재사용된다 — 편집 지점 아래의 어떤 실행도 그 이름들을
/// 건드리지 않기 때문이다. 반면 `gwas_df = pd.read_csv(...)`는 다시 돈다:
/// 밀려난 `gwas_df['locus_tag'].apply(...)`가 receiver를 바꿨을 수 있다.
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
