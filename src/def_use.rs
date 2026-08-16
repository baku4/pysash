use super::statement_facts::StatementFacts;

/// Tracks live names and time-indexed edges between possible aliases.
///
/// Alias closure uses only edges that existed at the queried time.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DefUseGraph {
    live: Vec<String>,
    edges: Vec<(String, String, usize)>,
}

impl DefUseGraph {
    /// Records bindings, deletions, and aliases from one execution.
    pub fn record(&mut self, facts: &StatementFacts, seq: usize) {
        for name in &facts.binds {
            if !self.live.iter().any(|live| **live == **name) {
                self.live.push(name.to_string());
            }
        }
        for name in &facts.deletes {
            self.live.retain(|live| **live != **name);
        }
        for (left, right) in &facts.alias_edges {
            self.edges.push((left.to_string(), right.to_string(), seq));
        }
    }

    pub fn live_names(&self) -> impl Iterator<Item = &str> {
        self.live.iter().map(String::as_str)
    }

    /// Adds aliases reachable through edges created before `before`.
    pub fn alias_closure(&self, names: &mut Vec<String>, before: usize) {
        loop {
            let mut grew = false;
            for (left, right, seq) in &self.edges {
                if *seq >= before {
                    continue;
                }
                let has_left = names.iter().any(|name| name == left);
                let has_right = names.iter().any(|name| name == right);
                if has_left && !has_right {
                    names.push(right.clone());
                    grew = true;
                } else if has_right && !has_left {
                    names.push(left.clone());
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DefUseGraph;
    use crate::statement_facts::StatementFacts;

    fn facts(
        binds: &[&str],
        deletes: &[&str],
        alias_edges: &[(&str, &str)],
    ) -> StatementFacts {
        StatementFacts {
            binds: binds.iter().map(|name| Box::from(*name)).collect(),
            deletes: deletes.iter().map(|name| Box::from(*name)).collect(),
            alias_edges: alias_edges
                .iter()
                .map(|(a, b)| (Box::from(*a), Box::from(*b)))
                .collect(),
            ..StatementFacts::default()
        }
    }

    #[test]
    fn live_names_follow_binds_and_deletes() {
        let mut graph = DefUseGraph::default();
        graph.record(&facts(&["x"], &[], &[]), 0);
        graph.record(&facts(&["y"], &[], &[]), 1);
        graph.record(&facts(&[], &["x"], &[]), 2);
        let live: Vec<&str> = graph.live_names().collect();
        assert_eq!(live, ["y"]);
    }

    #[test]
    fn alias_closure_is_transitive() {
        let mut graph = DefUseGraph::default();
        graph.record(&facts(&["b"], &[], &[("b", "a")]), 0);
        graph.record(&facts(&["c"], &[], &[("c", "b")]), 1);
        let mut names = vec!["a".to_string()];
        graph.alias_closure(&mut names, 10);
        names.sort_unstable();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn unrelated_names_stay_out_of_the_closure() {
        let mut graph = DefUseGraph::default();
        graph.record(&facts(&["b"], &[], &[("b", "a")]), 0);
        graph.record(&facts(&["d"], &[], &[("d", "c")]), 1);
        let mut names = vec!["a".to_string()];
        graph.alias_closure(&mut names, 10);
        names.sort_unstable();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn later_aliases_do_not_reach_back_in_time() {
        let mut graph = DefUseGraph::default();
        graph.record(&facts(&["p"], &[], &[("p", "a")]), 3);
        let mut names = vec!["a".to_string()];
        // A disturbance at sequence 2 cannot follow an alias created at sequence 3.
        graph.alias_closure(&mut names, 2);
        assert_eq!(names, ["a"]);
    }
}
