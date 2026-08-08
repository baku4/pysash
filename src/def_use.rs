use std::collections::HashMap;
use super::StatementFacts;

/// 이름 사이의 연결 기억 — 지금 어떤 이름이 살아 있고, 어떤 이름들이 같은 객체를
/// 가리킬 수 있는가.
///
/// 별칭 클래스는 늘어나기만 한다. 한 번이라도 `b = a`였다면 그 뒤로 b를 통한
/// 변경이 a에 보였을 수 있기 때문에, 별칭을 푸는 안전한 방법은 없다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DefUseGraph {
    /// 현재 바인딩되어 있는 이름들. 바인딩 순서를 유지한다.
    live: Vec<String>,
    /// union-find의 parent 포인터. 키에 없는 이름은 자기 자신이 뿌리다.
    parents: HashMap<String, String>,
}

impl DefUseGraph {
    /// 실행 하나가 남긴 바인딩·삭제·별칭을 기록한다.
    pub fn record(&mut self, facts: &StatementFacts) {
        for name in &facts.binds {
            if !self.live.iter().any(|live| **live == **name) {
                self.live.push(name.to_string());
            }
        }
        for name in &facts.deletes {
            self.live.retain(|live| **live != **name);
        }
        for (left, right) in &facts.alias_edges {
            self.union(left, right);
        }
    }

    pub fn live_names(&self) -> impl Iterator<Item = &str> {
        self.live.iter().map(String::as_str)
    }

    /// 주어진 이름들과 별칭 클래스가 겹치는 이름을 전부 추가한다.
    pub fn alias_closure(&self, names: &mut Vec<String>) {
        let roots: Vec<String> = names.iter().map(|name| self.find(name)).collect();
        for name in self.parents.keys() {
            if roots.contains(&self.find(name)) && !names.iter().any(|n| n == name) {
                names.push(name.clone());
            }
        }
    }

    fn union(&mut self, left: &str, right: &str) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents.insert(left_root, right_root.clone());
        }
        // 간선의 양 끝을 키로 등록해 둬야 closure가 이 이름들을 순회할 수 있다.
        self.parents.entry(left.to_string()).or_insert_with(|| right_root.clone());
        self.parents.entry(right.to_string()).or_insert(right_root);
    }

    fn find(&self, name: &str) -> String {
        let mut current = name;
        while let Some(parent) = self.parents.get(current) {
            if parent == current {
                break;
            }
            current = parent;
        }
        current.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::DefUseGraph;
    use crate::StatementFacts;

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
        graph.record(&facts(&["x"], &[], &[]));
        graph.record(&facts(&["y"], &[], &[]));
        graph.record(&facts(&[], &["x"], &[]));
        let live: Vec<&str> = graph.live_names().collect();
        assert_eq!(live, ["y"]);
    }

    #[test]
    fn alias_closure_is_transitive() {
        let mut graph = DefUseGraph::default();
        graph.record(&facts(&["b"], &[], &[("b", "a")]));
        graph.record(&facts(&["c"], &[], &[("c", "b")]));
        let mut names = vec!["a".to_string()];
        graph.alias_closure(&mut names);
        names.sort_unstable();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn unrelated_names_stay_out_of_the_closure() {
        let mut graph = DefUseGraph::default();
        graph.record(&facts(&["b"], &[], &[("b", "a")]));
        graph.record(&facts(&["d"], &[], &[("d", "c")]));
        let mut names = vec!["a".to_string()];
        graph.alias_closure(&mut names);
        names.sort_unstable();
        assert_eq!(names, ["a", "b"]);
    }
}
