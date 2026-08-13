use super::source::PythonSource;
use super::trace::ExecRef;
use super::SessionHistory;
use super::plan::Action;

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
            let seq = self.executions;
            self.executions += 1;
            self.realized.push(ExecRef { source, index, seq });
            self.graph.record(&statement.facts, seq);
            self.summaries.record(&statement.facts, seq);
        }
    }

    /// 이 소스의 [`align`](SessionHistory::align) 계획을 실행 완료했음을 기록한다.
    ///
    /// 이제 이 소스가 실현 열이 된다 — 실현 밖으로 밀려난 옛 실행들은 residue로
    /// 옮겨져 오염 계산의 재료로 남는다. plan을 인자로 받지 않고 내부에서 같은
    /// 판정을 다시 계산하므로, 위조된 plan이 세션을 오염시키는 경로가 없다.
    pub fn realize(&mut self, code: &PythonSource) {
        // 호출자가 본 것과 같은 계획. align은 순수하므로 결과가 같다.
        let plan = self.align(code);
        let statements = code.statements();
        let matched = plan.prefix_len;

        let source = self.sources.len();
        self.sources.push(code.clone());

        let displaced: Vec<ExecRef> = self.realized.drain(matched..).collect();
        self.residue.extend(displaced);

        // prefix 안에서 오염 때문에 Run이 된 것들은 방금 다시 실행되었다 — 그
        // 자리를 새 실행으로 바꿔 단다. 옛 실행을 그대로 두면 이미 지나간 오염이
        // 영원히 그 자리를 Run으로 만든다. 재사용된 것은 원래 실행 그대로다.
        for step in plan.steps.iter().take(matched) {
            if step.action == Action::Run {
                let index = step.index;
                let seq = self.executions;
                self.executions += 1;
                // canonical이 같으므로 graph/summaries에 더할 새 정보는 없다.
                self.realized[index] = ExecRef { source, index, seq };
            }
        }
        for (index, statement) in statements.iter().enumerate().skip(matched) {
            let seq = self.executions;
            self.executions += 1;
            self.realized.push(ExecRef { source, index, seq });
            self.graph.record(&statement.facts, seq);
            self.summaries.record(&statement.facts, seq);
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
