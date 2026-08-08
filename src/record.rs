use super::SessionHistory;
use super::python_source::PythonSource;
use super::trace::ExecRef;

impl SessionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// 성공한 실행 하나를 기록 끝에 잇는다.
    ///
    /// 세션에 실제로 입력해서 성공한 것만 넣는다. 실행하지 않은 것을 넣으면
    /// 기록이 거짓이 되고 판정도 따라서 틀린다.
    pub fn push(&mut self, code: &PythonSource) {
        let source = self.sources.len();
        self.sources.push(code.clone());
        for (index, statement) in code.statements().iter().enumerate() {
            let seq = self.realized.len() + self.residue.len();
            self.realized.push(ExecRef { source, index, seq });
            self.graph.record(&statement.facts);
            self.summaries.record(&statement.facts);
        }
    }

    /// 이 소스의 [`align`](SessionHistory::align) 계획을 실행 완료했음을 기록한다.
    ///
    /// 이제 이 소스가 실현 열이 된다 — 실현 밖으로 밀려난 옛 실행들은 residue로
    /// 옮겨져 오염 계산의 재료로 남는다. plan을 인자로 받지 않고 prefix를 내부에서
    /// 다시 계산하므로, 위조된 plan이 세션을 오염시키는 경로가 없다.
    pub fn realize(&mut self, code: &PythonSource) {
        let statements = code.statements();
        let matched = self
            .realized
            .iter()
            .zip(statements)
            .take_while(|(exec, statement)| {
                exec.statement(&self.sources).canonical == statement.canonical
            })
            .count();

        let source = self.sources.len();
        self.sources.push(code.clone());
        let displaced: Vec<ExecRef> = self.realized.drain(matched..).collect();
        self.residue.extend(displaced);
        for (index, statement) in statements.iter().enumerate().skip(matched) {
            let seq = self.realized.len() + self.residue.len();
            self.realized.push(ExecRef { source, index, seq });
            self.graph.record(&statement.facts);
            self.summaries.record(&statement.facts);
        }
    }

    /// 부분 실행 등으로 세션 상태를 더는 신뢰할 수 없다고 표시한다.
    ///
    /// 이후의 모든 align은 전부 Run을 낸다. 세션을 다시 신뢰하는 방법은 없다 —
    /// 인터프리터를 새로 띄우고 새 `SessionHistory`로 시작해야 한다.
    pub fn poison(&mut self) {
        self.poisoned = true;
    }

    /// 입력된 소스들. 순서 그대로다.
    pub fn sources(&self) -> &[PythonSource] {
        &self.sources
    }
}
