use super::statement_facts::StatementFacts;

/// 이름 사이의 연결 기억 — 지금 어떤 이름이 살아 있고, 어떤 이름들이 같은 객체를
/// 가리킬 수 있는가.
///
/// 별칭 간선은 생긴 실행 순번과 함께 쌓이기만 한다. 폐포는 언제나 "그 시점까지
/// 존재한 간선"으로만 계산한다 — 나중에 생긴 별칭이 그 전에 일어난 변경을 소급해서
/// 전파하면, 이미 지나간 실행의 오염이 부풀어 편집 루프가 수렴하지 않는다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DefUseGraph {
    /// 현재 바인딩되어 있는 이름들. 바인딩 순서를 유지한다.
    live: Vec<String>,
    /// 별칭 간선과 그것이 생긴 실행 순번.
    edges: Vec<(String, String, usize)>,
}

impl DefUseGraph {
    /// 실행 하나가 남긴 바인딩·삭제·별칭을 기록한다.
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

    /// `before` 이전에 존재한 별칭 간선만으로, 주어진 이름들과 같은 객체를 가리킬
    /// 수 있는 이름을 전부 추가한다.
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
        // 순번 2 시점의 변경은 아직 존재하지 않던 별칭을 타고 번지지 못한다.
        graph.alias_closure(&mut names, 2);
        assert_eq!(names, ["a"]);
    }
}
