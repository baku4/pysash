use std::collections::HashMap;
use super::disturbance::residue_entries;
use super::source::PythonSource;
use super::statement::Statement;
use super::summaries::SummaryTable;
use super::trace::ExecRef;

/// 어떤 판정에도 닿을 수 없게 된 실행을 실현 밖 열에서 버린다.
///
/// residue는 늘어나기만 하고 [`align`](crate::SessionHistory::align)은 매번 그 전체를
/// 훑는다. 오래 사는 세션에서는 판정 결과가 그대로인 채 비용만 자란다.
///
/// 버리는 것은 **판정에 닿을 수 없음이 증명되는 것뿐**이다. 히스토리를 고치는 것이
/// 아니므로 어떤 statement의 `action`도 이 호출로 바뀌지 않는다. 근거는 둘이고 둘 다
/// 오염을 좁히지 않는다.
///
/// 1. **실현 열 전체보다 앞선 실행은 영원히 무해하다.** 실현 밖 실행이 하는 일은
///    자기보다 순번이 앞선 실현 실행의 재사용을 깨는 것뿐인데(오염은 시간을 거슬러
///    일어나지 않는다), 실현 열의 모든 실행이 그보다 뒤 순번이면 지금 깰 것이 없다.
///    그리고 실현 열의 최소 순번은 절대 내려가지 않는다 — 재사용된 실행은 순번을
///    그대로 두고, 다시 실행된 것과 새로 붙는 것은 전부 더 뒤 순번을 받으며, 빠지는
///    것은 최소를 올릴 뿐이다. 그래서 앞으로도 깰 것이 없다.
/// 2. **오염 상계가 같은 실행이 뒤에 또 있으면 앞엣것은 덮인다.** 뒤엣것은 순번이
///    커서 더 많은 실행을 대상으로 삼고, 별칭 폐포도 그 시점까지 생긴 간선을 전부
///    보므로 더 넓다. 앞엣것이 걸리는 자리는 전부 뒤엣것도 걸린다.
///
/// 2의 상계는 시간이 지나도 변하지 않는다. 요약 조회는 실행 순번보다 앞선 최신 정의를
/// 쓰는데, 앞으로 기록될 요약은 전부 더 뒤 순번이라 이미 지나간 해석을 바꾸지 못한다.
///
/// **statement가 달라도 상계가 같으면 덮인다** — 같은 이름을 다시 묶기만 하는 `x = 1`과
/// `x = 2`가 그렇다. 한 줄을 고쳐 가며 반복 실행하는 편집 루프에서 실현 밖으로 밀려나
/// residue를 채우는 것이 정확히 그 모양이므로, 여기서 사이클당 증가가 사라진다.
///
/// 버리는 것은 실행 참조뿐이다. def-use 그래프와 요약 표는 그대로 둔다 — 그쪽은
/// `produces`를 넓히는 재료이고, 지우면 상계가 좁아져 잘못된 재사용이 된다.
pub fn forget_inert(
    residue: &mut Vec<ExecRef>,
    realized: &[ExecRef],
    sources: &[PythonSource],
    summaries: &SummaryTable,
) {
    // 판정 대상이 될 수 있는 가장 앞선 순번. 실현 열이 비어 있으면 재사용의 근거로
    // 삼을 실행이 아예 없으므로 실현 밖 열 전체가 죽은 기록이다.
    let Some(floor) = realized.iter().map(|exec| exec.seq).min() else {
        residue.clear();
        return;
    };

    let outside: Vec<(usize, &Statement)> = residue
        .iter()
        .map(|exec| (exec.seq, exec.statement(sources)))
        .collect();
    let entries = residue_entries(&outside, summaries);

    // 순번을 뺀 오염 상계마다 가장 뒤 순번. 그것만 남으면 나머지는 덮인다. 이름
    // 목록은 순서까지 같아야 같은 것으로 본다 — 같은 집합을 다른 순서로 잡은 둘은
    // 덮이지 않고 그냥 남는다. 버리지 못할 뿐 틀리지 않는다.
    let mut newest: HashMap<(&[String], &[String], bool), usize> = HashMap::new();
    for entry in &entries {
        let bound = (&entry.rebound[..], &entry.mutated[..], entry.opaque);
        newest
            .entry(bound)
            .and_modify(|seq| *seq = (*seq).max(entry.seq))
            .or_insert(entry.seq);
    }

    let live: Vec<bool> = entries
        .iter()
        .map(|entry| {
            let bound = (&entry.rebound[..], &entry.mutated[..], entry.opaque);
            entry.seq > floor && newest[&bound] == entry.seq
        })
        .collect();

    *residue = std::mem::take(residue)
        .into_iter()
        .zip(live)
        .filter_map(|(exec, live)| live.then_some(exec))
        .collect();
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use crate::SessionHistory;
    use crate::plan::Action;
    use crate::source::PythonSource;
    use crate::trace::ExecRef;

    /// 바인딩·mutation·별칭·전이 global 쓰기·재정의가 섞인 어휘. 실현 열의 앞이
    /// 재사용으로 남아야 실현 밖 열이 실제로 쌓이므로, 아무것도 건드리지 않는
    /// statement도 섞여 있어야 한다.
    const VOCAB: &[&str] = &[
        "x = 1\n",
        "y = x\n",
        "a = []\n",
        "a.append(1)\n",
        "keep = a\n",
        "c = 0\n",
        "def bump():\n    global c\n    c = c + 1\n",
        "def bump():\n    pass\n",
        "bump()\n",
        "import os\n",
        "n = len(a)\n",
    ];

    fn source(picks: &[usize]) -> PythonSource {
        let text: String = picks.iter().map(|pick| VOCAB[*pick]).collect();
        PythonSource::parse(&text).expect("vocab is valid python")
    }

    fn actions(history: &SessionHistory, code: &PythonSource) -> Vec<Action> {
        let plan = history.align(code);
        plan.steps.iter().map(|step| step.action).collect()
    }

    /// 실현 열을 그대로 옮겨 적은 소스. 이걸로 정렬하면 prefix가 최대라 실현 열의
    /// **모든** 실행이 판정 대상이 된다 — 순번 비교가 어긋나면 여기서 드러난다.
    fn mirror(history: &SessionHistory) -> PythonSource {
        let text: String = history
            .realized
            .iter()
            .map(|exec| {
                let source = &history.sources[exec.source];
                let slice = source.slice(exec.statement(&history.sources).range);
                let text = str::from_utf8(slice).expect("statement 경계는 문자 경계다");
                format!("{text}\n")
            })
            .collect();
        PythonSource::parse(&text).expect("statement를 이어 붙인 것도 python이다")
    }

    /// **압축은 판정을 바꾸지 않는다.**
    ///
    /// 같은 호출 열을 두 세션에 똑같이 먹이되, 한쪽은 압축이 버린 것을 도로 붙여
    /// "압축이 없었던 세션"으로 유지한다. 압축은 실현 밖 열만 건드리므로 나머지는
    /// 저절로 lockstep이다. 두 세션의 판정이 어떤 소스에 대해서도 같아야 한다 —
    /// 필요한 것을 하나라도 버렸다면 그 자리에서 Run이 Reuse로 바뀐다.
    ///
    /// `push`를 섞는 것이 중요하다. `realize`만 반복하면 실현 열의 순번이 아주
    /// 낮거나(계속 재사용된 앞부분) 실현 밖 열 전체보다 높거나(방금 다시 실행된
    /// 부분) 둘 중 하나라 중간 대역이 생기지 않고, 순번 비교가 어긋나도 드러나지
    /// 않는다.
    #[test]
    fn forgetting_never_changes_a_verdict() {
        let script = proptest::collection::vec(
            (0..3usize, proptest::collection::vec(0..VOCAB.len(), 0..5)),
            1..8,
        );
        proptest!(|(script in script)| {
            let mut compacted = SessionHistory::new();
            let mut full = SessionHistory::new();
            let mut seen: Vec<PythonSource> = Vec::new();

            for (kind, picks) in &script {
                let code = source(picks);
                seen.push(code.clone());
                let kept = full.residue.clone();
                match kind {
                    0 => {
                        compacted.push(&code);
                        full.push(&code);
                        continue;
                    }
                    1 => {
                        compacted.realize(&code);
                        // realize가 실현 밖으로 민 것.
                        let matched = full.align(&code).prefix_len;
                        let displaced: Vec<ExecRef> = full.realized[matched..].to_vec();
                        full.realize(&code);
                        full.residue = kept.into_iter().chain(displaced).collect();
                    }
                    _ => {
                        compacted.record_partial(&code);
                        // record_partial이 실현 밖에 붙인 것.
                        let at = full.sources.len();
                        let first = full.executions;
                        full.record_partial(&code);
                        let added = (0..code.statements().len()).map(|index| ExecRef {
                            source: at,
                            index,
                            seq: first + index,
                        });
                        full.residue = kept.into_iter().chain(added).collect();
                    }
                }
            }

            seen.push(mirror(&compacted));
            for code in &seen {
                prop_assert_eq!(actions(&compacted, code), actions(&full, code));
            }
            prop_assert!(compacted.residue_count() <= full.residue_count());
        });
    }

    /// 실현 열이 비면 재사용의 근거가 될 실행이 없다 — 실현 밖 열 전체가 죽은 기록이다.
    #[test]
    fn an_empty_realized_column_leaves_nothing_reachable() {
        let mut history = SessionHistory::new();
        history.record_partial(&PythonSource::parse("x = 1\n").expect("valid python"));
        assert_eq!(history.residue_count(), 0);
    }
}
