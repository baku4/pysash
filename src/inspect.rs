use super::SessionHistory;

impl SessionHistory {
    /// 실현 열의 statement 수.
    pub fn statement_count(&self) -> usize {
        self.realized.len()
    }

    /// 실현 열 밖으로 밀려난 실행의 수. 0이면 세션의 모든 실행이 마지막으로
    /// 정렬된 소스의 실행으로 세어진다.
    pub fn residue_count(&self) -> usize {
        self.residue.len()
    }

    /// 현재 바인딩되어 있는 이름들.
    pub fn live_names(&self) -> impl Iterator<Item = &str> {
        self.graph.live_names()
    }
}
