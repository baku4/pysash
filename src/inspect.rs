use super::SessionHistory;

impl SessionHistory {
    /// 실현 열의 statement 수.
    pub fn statement_count(&self) -> usize {
        self.realized.len()
    }

    /// 세션이 들고 있는 실현 밖 실행의 수.
    ///
    /// 밀려난 실행의 **누계가 아니다.** 어떤 판정에도 닿을 수 없게 된 실행은 세션이
    /// 버리므로, 편집-실행을 반복해도 이 수는 자라기만 하지 않는다. 0이면 실현 열
    /// 밖에서 앞으로의 판정에 영향을 줄 수 있는 실행이 없다는 뜻이다.
    pub fn residue_count(&self) -> usize {
        self.residue.len()
    }

    /// 현재 바인딩되어 있는 이름들.
    pub fn live_names(&self) -> impl Iterator<Item = &str> {
        self.graph.live_names()
    }
}
