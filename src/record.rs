use super::forget::forget_inert;
use super::source::PythonSource;
use super::statement::Statement;
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
    /// 기록이 거짓이 되고 판정도 따라서 틀린다. 어디까지 돌았는지 모르는 실행은
    /// [`record_partial`](Self::record_partial)이 받는다.
    pub fn push(&mut self, code: &PythonSource) {
        let source = self.sources.len();
        self.sources.push(code.clone());
        for (index, statement) in code.statements().iter().enumerate() {
            let exec = self.record_execution(source, index, statement);
            self.realized.push(exec);
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
                let exec = self.record_execution(source, index, &statements[index]);
                self.realized[index] = exec;
            }
        }
        for (index, statement) in statements.iter().enumerate().skip(matched) {
            let exec = self.record_execution(source, index, statement);
            self.realized.push(exec);
        }
        forget_inert(
            &mut self.residue,
            &self.realized,
            &self.sources,
            &self.summaries,
        );
    }

    /// 어디까지 돌았는지 모르는 실행 하나를 기록한다.
    ///
    /// 실행이 중간에 끊겼을 때 쓴다 — 예외로 멈췄거나, 사용자가 취소했거나,
    /// 인터프리터가 죽었을 때다. 소스를 **통째로** residue에 넣는다: 효과는 남아
    /// 있지만 더 이상 어떤 소스의 실행으로도 세지 않는다는 뜻이고, 그래서 이
    /// 소스는 재사용의 근거가 되지 못하면서 오염 계산에는 전부 들어간다.
    ///
    /// 실제로 돈 것보다 넓게 기록하는 방향이므로 오염 상계는 넓어질 뿐 좁아지지
    /// 않는다. 넓은 오염은 Run이 늘어날 뿐이다.
    ///
    /// [`poison`](Self::poison)과는 다른 상황이다. poison은 세션에 무슨 일이
    /// 있었는지 **아무것도** 모를 때의 값이고, 여기서는 문제가 될 수 있는
    /// statement의 집합을 정확히 알고 있다. 아는 만큼만 버리면 그 소스가 건드리지
    /// 않은 이름들의 재사용은 살아남는다.
    ///
    /// 완주한 부분이 있으면 그쪽을 [`push`](Self::push)나
    /// [`realize`](Self::realize)로 **먼저** 기록한다. 끊긴 실행이 뒤 순번을 받아야
    /// 오염이 시간을 거스르지 않는다.
    pub fn record_partial(&mut self, code: &PythonSource) {
        let source = self.sources.len();
        self.sources.push(code.clone());
        for (index, statement) in code.statements().iter().enumerate() {
            let exec = self.record_execution(source, index, statement);
            self.residue.push(exec);
        }
        forget_inert(
            &mut self.residue,
            &self.realized,
            &self.sources,
            &self.summaries,
        );
    }

    /// 세션에 무슨 일이 있었는지 알 수 없다고 표시한다.
    ///
    /// 이후의 모든 align은 전부 Run을 낸다. 세션을 다시 신뢰하는 방법은 없다 —
    /// 인터프리터를 새로 띄우고 새 `SessionHistory`로 시작해야 한다.
    ///
    /// 어떤 소스가 돌다 끊겼는지 아는 경우라면 이게 아니라
    /// [`record_partial`](Self::record_partial)이다. 세션 하나를 통째로 버리는
    /// 값은 소스조차 모를 때를 위한 것이다.
    pub fn poison(&mut self) {
        self.poisoned = true;
    }

    /// 입력된 소스들. 순서 그대로다.
    pub fn sources(&self) -> &[PythonSource] {
        &self.sources
    }

    /// 실행 하나에 순번을 발급하고 그 효과를 def-use 그래프와 요약 표에 남긴다.
    /// 실현 열에 놓을지 residue에 놓을지는 호출부가 정한다.
    ///
    /// 실행을 만드는 유일한 자리다. 순번만 받고 그래프·요약 기록을 건너뛴 실행이
    /// 생기면, 그 실행이 다시 정의한 이름의 상계가 그 자리에서 낡는다 — 사이에
    /// 끼어든 옛 재정의가 이 실행보다 뒤에 놓인 호출에 그대로 남아 상계를 좁힌다.
    /// 좁아진 상계는 잘못된 재사용이다.
    fn record_execution(&mut self, source: usize, index: usize, statement: &Statement) -> ExecRef {
        let seq = self.executions;
        self.executions += 1;
        self.graph.record(&statement.facts, seq);
        self.summaries.record(&statement.facts, seq);
        ExecRef { source, index, seq }
    }
}
