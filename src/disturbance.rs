use super::def_use::DefUseGraph;
use super::statement::Statement;
use super::summaries::SummaryTable;

/// 실현 밖 실행 하나가 세션 상태에서 망가뜨렸을 수 있는 것의 상계.
pub struct ResidueEntry {
    /// 이 실행의 전역 실행 순서. 이보다 먼저 일어난 실행만 오염시킬 수 있다 —
    /// 오염은 시간을 거슬러 일어나지 않는다.
    pub seq: usize,
    /// 다시 바인딩했거나 지운 이름들 (호출한 세션 정의 함수의 전이 global 쓰기 포함).
    pub rebound: Vec<String>,
    /// in-place로 변경했을 수 있는 이름들.
    pub mutated: Vec<String>,
    /// 무엇을 건드렸는지 알 수 없는 실행 — 자기보다 앞선 실행 전부를 오염시킨다.
    pub opaque: bool,
}

/// 오염이 statement의 실행 효과와 겹치는 지점.
pub enum Hit {
    /// 이 statement가 남긴 이름이 그 뒤에 다시 바인딩되었다.
    Rebound(String),
    /// 이 statement가 남긴 객체가 그 뒤에 변경되었을 수 있다.
    Mutated(String),
    /// 그 뒤의 어떤 실행이 무엇을 건드렸는지 알 수 없다.
    Opaque,
}

/// residue 실행들 각각이 망가뜨렸을 수 있는 이름을 모은다.
pub fn residue_entries(
    residue: &[(usize, &Statement)],
    summaries: &SummaryTable,
) -> Vec<ResidueEntry> {
    residue
        .iter()
        .map(|(seq, statement)| {
            let facts = &statement.facts;
            let mut entry = ResidueEntry {
                seq: *seq,
                rebound: Vec::new(),
                mutated: Vec::new(),
                opaque: facts.opaque,
            };
            for name in facts.binds.iter().chain(&facts.deletes) {
                push_unique(&mut entry.rebound, name);
            }
            for name in &facts.mutates {
                push_unique(&mut entry.mutated, name);
            }
            for call in &facts.calls {
                // 이 실행은 자기 시점에 살아 있던 정의를 호출했다.
                if let Some(summary) = summaries.resolve(call, *seq) {
                    entry.opaque |= summary.opaque;
                    for name in &summary.global_writes {
                        push_unique(&mut entry.rebound, name);
                    }
                    for name in &summary.mutates_frees {
                        push_unique(&mut entry.mutated, name);
                    }
                }
            }
            entry
        })
        .collect()
}

/// `seq` 시점의 실행이 남긴 것을, 그보다 뒤의 residue 실행이 건드렸는가.
pub fn hits(
    entries: &[ResidueEntry],
    seq: usize,
    statement: &Statement,
    summaries: &SummaryTable,
    graph: &DefUseGraph,
) -> Option<Hit> {
    let facts = &statement.facts;

    // 이 statement의 실행이 남긴 것 — 바인딩한 이름과, 만들거나 변경했을 수 있는 객체.
    let mut produces: Vec<String> = Vec::new();
    for name in facts
        .binds
        .iter()
        .chain(&facts.deletes)
        .chain(&facts.mutates)
    {
        push_unique(&mut produces, name);
    }
    for call in &facts.calls {
        // 이 실행은 자기 시점에 살아 있던 정의를 호출했다.
        if let Some(summary) = summaries.resolve(call, seq) {
            for name in summary.global_writes.iter().chain(&summary.mutates_frees) {
                push_unique(&mut produces, name);
            }
        }
    }

    for entry in entries.iter().filter(|entry| entry.seq > seq) {
        if entry.opaque {
            return Some(Hit::Opaque);
        }
        for name in &produces {
            if entry.rebound.contains(name) {
                return Some(Hit::Rebound(name.clone()));
            }
        }
        // 별칭 폐포는 in-place 변경 쪽에만 의미가 있고 (이름 재바인딩은 별칭이
        // 가리키던 객체를 건드리지 않는다), 이 entry의 변경 시점까지 존재한
        // 별칭으로만 번진다.
        let mut reachable = produces.clone();
        graph.alias_closure(&mut reachable, entry.seq);
        for name in &reachable {
            if entry.mutated.contains(name) {
                return Some(Hit::Mutated(name.clone()));
            }
        }
    }
    None
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|existing| existing == name) {
        names.push(name.to_string());
    }
}
