//! fixture 로더와, 판정 결과를 한 줄로 읽게 만드는 도우미.
//!
//! 통합 테스트는 crate가 따로 잡히므로 여기 있는 함수 중 일부는 어느 한쪽에서만
//! 쓰인다. 그 쪽의 dead_code 경고를 여기서 한 번에 끈다.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use pysash::SessionHistory;
use pysash::plan::{Action, AlignmentPlan, DecisionReason, Diagnostic};
use pysash::source::PythonSource;

/// 원본 텍스트를 함께 들고 다니는 fixture. 실패 메시지에 이름을 붙이려고 있다.
pub struct Fixture {
    pub name: String,
    pub text: String,
    pub source: PythonSource,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(path: &Path, name: String) -> Fixture {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let source = PythonSource::parse(&text)
        .unwrap_or_else(|e| panic!("{} is not parseable python: {e}", path.display()));
    Fixture { name, text, source }
}

/// `tests/fixtures/corpus/`의 모든 소스. 이름 순으로 고정된다.
pub fn corpus() -> Vec<Fixture> {
    let dir = fixtures_dir().join("corpus");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "py"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "corpus가 비어 있다: {}", dir.display());
    paths
        .iter()
        .map(|path| {
            let name = path.file_name().expect("file").to_string_lossy().into_owned();
            load(path, name)
        })
        .collect()
}

/// `tests/fixtures/sessions/<scenario>/<file>`.
pub fn step(scenario: &str, file: &str) -> PythonSource {
    let path = fixtures_dir().join("sessions").join(scenario).join(file);
    load(&path, format!("{scenario}/{file}")).source
}

/// 판정을 한 줄 문자열로. `.`은 Reuse, `X`는 Run이다.
///
/// 열 개 넘는 statement의 판정을 `Vec<Action>`으로 비교하면 어디가 어긋났는지
/// 눈으로 못 찾는다. 자리마다 한 글자면 index를 세어서 찾을 수 있다.
pub fn actions(plan: &AlignmentPlan) -> String {
    plan.plans
        .iter()
        .map(|statement| match statement.action {
            Action::Reuse => '.',
            Action::Run => 'X',
        })
        .collect()
}

pub fn reasons(plan: &AlignmentPlan) -> Vec<DecisionReason> {
    plan.plans
        .iter()
        .map(|statement| statement.reason.clone())
        .collect()
}

/// 판정과 원문 첫 줄을 나란히 — 기대값이 어긋났을 때 눈으로 확인하는 용도다.
pub fn explain(plan: &AlignmentPlan, code: &PythonSource) -> String {
    let mut out = String::new();
    for statement in &plan.plans {
        let text = String::from_utf8_lossy(code.slice(statement.range));
        let first_line = text.lines().next().unwrap_or("").trim_end();
        out.push_str(&format!(
            "{:>3} {} {:<28} {first_line}\n",
            statement.index,
            match statement.action {
                Action::Reuse => '.',
                Action::Run => 'X',
            },
            format!("{:?}", statement.reason),
        ));
    }
    out
}

pub fn has_diagnostic(plan: &AlignmentPlan, matches: impl Fn(&Diagnostic) -> bool) -> bool {
    plan.diagnostics.iter().any(matches)
}

/// 앞에서부터 `count`개 statement만 남긴 소스. 원본 바이트를 그대로 자른다.
///
/// top-level statement는 전부 줄 첫 칸에서 시작하므로 statement 끝에서 자르면
/// 언제나 파싱된다.
pub fn head(source: &PythonSource, count: usize) -> PythonSource {
    assert!(count > 0 && count <= source.statements().len());
    let end = source.statements()[count - 1].range.end as usize;
    PythonSource::parse_bytes(&source.raw()[..end]).expect("statement 경계에서 자른 소스")
}

/// `index` 자리에 새 statement 한 줄을 끼워 넣은 소스.
///
/// 어떤 코드든 안전하게 "중간을 고친" 판본을 만들 수 있는 유일한 편집이다 —
/// 이름이 겹치지 않고, 들여쓰기를 건드리지 않으며, 아래 statement를 한 칸씩
/// 밀기만 한다.
pub fn insert_probe(source: &PythonSource, index: usize) -> PythonSource {
    let raw = source.raw();
    let at = source.statements()[index].range.start as usize;
    assert!(
        at == 0 || raw[at - 1] == b'\n',
        "statement {index}가 줄 첫 칸에서 시작하지 않는다"
    );
    let mut bytes = Vec::with_capacity(raw.len() + PROBE.len());
    bytes.extend_from_slice(&raw[..at]);
    bytes.extend_from_slice(PROBE.as_bytes());
    bytes.extend_from_slice(&raw[at..]);
    PythonSource::parse_bytes(&bytes).expect("한 줄 삽입은 파싱을 깨지 않는다")
}

/// `index` 자리의 statement를 다른 한 줄로 갈아 끼운 소스. statement 수는 그대로다.
///
/// 셀을 새로 만들지도, 아래를 밀지도 않는 **제자리 수정** — 셸에서 한 줄 고치는
/// 바로 그 편집이다. 아래 statement들은 index도 canonical도 그대로지만, 앞이
/// 달라졌으므로 더 이상 "그 자리의 실행"이 아니다.
pub fn replace_statement(source: &PythonSource, index: usize) -> PythonSource {
    let raw = source.raw();
    let statement = &source.statements()[index];
    let (start, end) = (statement.range.start as usize, statement.range.end as usize);
    assert!(
        start == 0 || raw[start - 1] == b'\n',
        "statement {index}가 줄 첫 칸에서 시작하지 않는다"
    );
    let mut bytes = Vec::with_capacity(raw.len() + PROBE.len());
    bytes.extend_from_slice(&raw[..start]);
    // 개행은 원문의 것을 그대로 쓴다 — 뒤따르는 주석도 그 자리에 남는다.
    bytes.extend_from_slice(PROBE.trim_end().as_bytes());
    bytes.extend_from_slice(&raw[end..]);
    PythonSource::parse_bytes(&bytes).expect("한 줄 치환은 파싱을 깨지 않는다")
}

/// 각 statement가 jupytext `# %%` 셀 중 몇 번째에 들어 있는가.
///
/// 셸은 셀 단위로 실행한다 — 셀 안에 Run이 하나라도 있으면 그 셀은 통째로 돈다.
/// 마커가 없는 소스는 전부 한 셀로 본다.
pub fn cell_of_statement(source: &PythonSource) -> Vec<usize> {
    let raw = source.raw();
    let mut marks: Vec<usize> = Vec::new();
    let mut offset = 0;
    for line in raw.split_inclusive(|byte| *byte == b'\n') {
        if line.starts_with(b"# %%") {
            marks.push(offset);
        }
        offset += line.len();
    }
    source
        .statements()
        .iter()
        .map(|statement| {
            let at = statement.range.start as usize;
            marks.iter().rposition(|mark| *mark <= at).unwrap_or(0)
        })
        .collect()
}

/// 셀 단위 재사용 — (다시 돌지 않는 셀, statement가 든 셀 전체).
///
/// `cells`는 plan을 낳은 그 소스의 [`cell_of_statement`] 결과여야 한다.
pub fn cell_reuse(plan: &AlignmentPlan, cells: &[usize]) -> (usize, usize) {
    let mut run: Vec<usize> = plan
        .plans
        .iter()
        .filter(|statement| statement.action == Action::Run)
        .map(|statement| cells[statement.index])
        .collect();
    run.sort_unstable();
    run.dedup();

    let mut all = cells.to_vec();
    all.sort_unstable();
    all.dedup();
    (all.len() - run.len(), all.len())
}

/// 맨 뒤에 statement 한 줄을 덧붙인 소스.
pub fn append_probe(source: &PythonSource) -> PythonSource {
    let mut bytes = source.raw().to_vec();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(PROBE.as_bytes());
    PythonSource::parse_bytes(&bytes).expect("한 줄 덧붙이기는 파싱을 깨지 않는다")
}

/// corpus 어디에도 나오지 않는 이름이어야 한다 — 겹치면 canonical이 우연히
/// 일치해 prefix가 밀린다.
const PROBE: &str = "pysash_probe_marker = 1\n";

/// 이 소스만 실행한 세션.
pub fn realized(source: &PythonSource) -> SessionHistory {
    let mut history = SessionHistory::new();
    history.realize(source);
    history
}
