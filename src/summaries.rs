use std::collections::{HashMap, HashSet};
use super::statement_facts::{CalleeSummary, StatementFacts};

/// 세션에 정의된 callable들의 요약. 정의된 실행 순번과 함께 버전으로 쌓인다.
///
/// 호출은 **호출 시점에 살아 있던 정의**를 실행한다 (late binding). 그래서 조회에는
/// 시점이 붙는다 — 나중의 재정의가 먼저 일어난 호출의 효과를 소급해서 바꾸면,
/// 이미 지나간 실행의 상계가 부풀어 편집 루프가 수렴하지 않는다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SummaryTable {
    /// 이름 → (정의된 실행 순번, 요약)의 버전들. 순번 오름차순으로 쌓인다.
    by_name: HashMap<String, Vec<(usize, CalleeSummary)>>,
}

impl SummaryTable {
    /// 실행 하나가 callable을 정의했다면 그 요약을 바인딩 이름마다 기록한다.
    pub fn record(&mut self, facts: &StatementFacts, seq: usize) {
        let Some(summary) = &facts.summary else {
            return;
        };
        for name in &facts.binds {
            self.by_name
                .entry(name.to_string())
                .or_default()
                .push((seq, summary.clone()));
        }
    }

    /// `at` 시점의 호출이 이 이름으로 일으킬 수 있는 일의 전이 폐포.
    ///
    /// 각 이름은 `at`보다 먼저 기록된 최신 정의로 해석된다. 세션에 정의가 없는
    /// 이름이면 None — 외부에서 온 함수는 이 module의 global을 재바인딩하지
    /// 않는다고 가정한다 (A-NoForeignGlobalWrite). 재귀·상호재귀는 방문 집합으로
    /// 끝난다.
    pub fn resolve(&self, name: &str, at: usize) -> Option<CalleeSummary> {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut stack = vec![name];
        let mut resolved = CalleeSummary::default();
        let mut found = false;
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            let Some(versions) = self.by_name.get(current) else {
                continue;
            };
            let Some((_, summary)) = versions.iter().rev().find(|(seq, _)| *seq < at) else {
                continue;
            };
            found = true;
            resolved.absorb(summary);
            for callee in &summary.callees {
                if let Some((key, _)) = self.by_name.get_key_value(&**callee) {
                    stack.push(key);
                }
            }
        }
        found.then_some(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::SummaryTable;
    use crate::statement_facts::{CalleeSummary, StatementFacts};

    fn def(name: &str, summary: CalleeSummary) -> StatementFacts {
        StatementFacts {
            binds: vec![Box::from(name)],
            summary: Some(summary),
            ..StatementFacts::default()
        }
    }

    fn writes_global(name: &str) -> CalleeSummary {
        CalleeSummary {
            global_writes: vec![Box::from(name)],
            ..CalleeSummary::default()
        }
    }

    fn calls(names: &[&str]) -> CalleeSummary {
        CalleeSummary {
            callees: names.iter().map(|name| Box::from(*name)).collect(),
            ..CalleeSummary::default()
        }
    }

    #[test]
    fn transitive_global_writes_are_resolved() {
        let mut table = SummaryTable::default();
        table.record(&def("g", writes_global("c")), 0);
        table.record(&def("h", calls(&["g"])), 1);
        let resolved = table.resolve("h", 10).expect("h is defined");
        assert_eq!(&*resolved.global_writes[0], "c");
    }

    #[test]
    fn mutual_recursion_terminates() {
        let mut table = SummaryTable::default();
        table.record(&def("a", calls(&["b"])), 0);
        table.record(&def("b", calls(&["a"])), 1);
        assert!(table.resolve("a", 10).is_some());
    }

    #[test]
    fn unknown_names_resolve_to_none() {
        let table = SummaryTable::default();
        assert!(table.resolve("imported", 10).is_none());
    }

    #[test]
    fn resolution_is_time_aware() {
        let mut table = SummaryTable::default();
        table.record(&def("f", writes_global("c")), 1);
        table.record(&def("f", writes_global("d")), 5);
        // 정의되기 전의 호출은 이 정의를 실행했을 수 없다.
        assert!(table.resolve("f", 0).is_none());
        // 사이의 호출은 첫 번째 정의를, 나중의 호출은 재정의를 실행했다.
        assert_eq!(&*table.resolve("f", 3).unwrap().global_writes[0], "c");
        assert_eq!(&*table.resolve("f", 9).unwrap().global_writes[0], "d");
    }
}
