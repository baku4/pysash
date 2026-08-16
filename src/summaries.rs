use std::collections::{HashMap, HashSet};
use super::statement_facts::{CalleeSummary, StatementFacts};

/// Time-indexed effect summaries for callables defined in the session.
///
/// Resolution uses the latest definition preceding the call sequence.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SummaryTable {
    by_name: HashMap<String, Vec<(usize, CalleeSummary)>>,
}

impl SummaryTable {
    /// Records a callable summary for every name bound by an execution.
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

    /// Resolves the transitive effects of calling `name` at sequence `at`.
    ///
    /// Returns `None` when no preceding session definition is known.
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
        // Calls before a definition cannot resolve to it.
        assert!(table.resolve("f", 0).is_none());
        // Calls resolve to the latest definition available at their sequence.
        assert_eq!(&*table.resolve("f", 3).unwrap().global_writes[0], "c");
        assert_eq!(&*table.resolve("f", 9).unwrap().global_writes[0], "d");
    }
}
