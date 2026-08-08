use std::collections::{HashMap, HashSet};
use super::{CalleeSummary, StatementFacts};

/// 세션에 정의된 callable들의 요약.
///
/// 요약은 이름에 합쳐지기만 한다. 같은 이름이 다른 내용으로 다시 정의되면 두 요약의
/// 합집합이 된다 — 기록의 어느 시점에 어느 버전이 불렸는지 되짚는 대신, 언제 불렸어도
/// 일어날 수 있었던 일의 상계를 유지한다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SummaryTable {
    by_name: HashMap<String, CalleeSummary>,
}

impl SummaryTable {
    /// 실행 하나가 callable을 정의했다면 그 요약을 바인딩 이름마다 기록한다.
    pub fn record(&mut self, facts: &StatementFacts) {
        let Some(summary) = &facts.summary else {
            return;
        };
        for name in &facts.binds {
            self.by_name
                .entry(name.to_string())
                .and_modify(|existing| existing.absorb(summary))
                .or_insert_with(|| summary.clone());
        }
    }

    /// 이 이름을 호출하면 일어날 수 있는 일의 전이 폐포.
    ///
    /// 세션에 정의가 없는 이름이면 None — 외부에서 온 함수는 이 module의 global을
    /// 재바인딩하지 않는다고 가정한다 (A-NoForeignGlobalWrite). 재귀·상호재귀는
    /// 방문 집합으로 끝난다.
    pub fn resolve(&self, name: &str) -> Option<CalleeSummary> {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut stack = vec![name];
        let mut resolved = CalleeSummary::default();
        let mut found = false;
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            if let Some(summary) = self.by_name.get(current) {
                found = true;
                resolved.absorb(summary);
                for callee in &summary.callees {
                    if let Some((key, _)) = self.by_name.get_key_value(&**callee) {
                        stack.push(key);
                    }
                }
            }
        }
        found.then_some(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::SummaryTable;
    use crate::{CalleeSummary, StatementFacts};

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
        table.record(&def("g", writes_global("c")));
        table.record(&def("h", calls(&["g"])));
        let resolved = table.resolve("h").expect("h is defined");
        assert_eq!(&*resolved.global_writes[0], "c");
    }

    #[test]
    fn mutual_recursion_terminates() {
        let mut table = SummaryTable::default();
        table.record(&def("a", calls(&["b"])));
        table.record(&def("b", calls(&["a"])));
        assert!(table.resolve("a").is_some());
    }

    #[test]
    fn unknown_names_resolve_to_none() {
        let table = SummaryTable::default();
        assert!(table.resolve("imported").is_none());
    }

    #[test]
    fn redefinition_unions_instead_of_replacing() {
        let mut table = SummaryTable::default();
        table.record(&def("f", writes_global("c")));
        table.record(&def("f", writes_global("d")));
        let resolved = table.resolve("f").expect("f is defined");
        let mut writes: Vec<&str> = resolved.global_writes.iter().map(|n| &**n).collect();
        writes.sort_unstable();
        assert_eq!(writes, ["c", "d"]);
    }
}
