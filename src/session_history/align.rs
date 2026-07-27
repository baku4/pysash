use crate::alignment_plan::{AlignmentPlan, StatementPlan};
use crate::decision_reason::{Action, DecisionReason};
use crate::python_source::PythonSource;
use crate::statement::Statement;
use super::SessionHistory;
use super::overlap::overlap_len;

impl SessionHistory {
    /// 이 소스를 실행하려면 무엇을 재사용하고 무엇을 다시 실행해야 하는가.
    ///
    /// 세션을 바꾸지 않으며 실패하지 않는다 — 전부 다시 실행하라는 것도 유효한
    /// 계획이기 때문이다.
    ///
    /// 재사용의 근거는 하나뿐이다: **세션의 끝이 이 소스의 앞과 이어진다.** 세션의
    /// 마지막 `m`개가 이 소스의 첫 `m`개와 같으면 그 `m`개는 방금 이 소스를 그만큼
    /// 실행한 것이고 그 뒤에 아무 일도 없었다. 그러므로 나머지를 순서대로 실행하면
    /// 이 소스를 통째로 실행한 것과 같은 상태가 된다.
    ///
    /// 세션이 이 소스와 갈라진 뒤에도 계속 실행했다면 꼬리가 이어지지 않으므로
    /// 재사용이 없다. 되돌릴 수 없는 실행 위에서 그보다 나은 답은 없다.
    pub fn align(&self, code: &PythonSource) -> AlignmentPlan {
        let session: Vec<&Statement> = self
            .sources
            .iter()
            .flat_map(|source| source.statements())
            .collect();
        let statements = code.statements();

        // 판정: 세션의 끝과 소스의 앞이 겹치는 만큼.
        let reused = overlap_len(&session, statements);
        // 이유 라벨링에만 쓴다: 세션이 이 소스와 앞에서부터 갈라지는 자리.
        let diverged_at = session
            .iter()
            .zip(statements)
            .take_while(|(a, b)| a.canonical == b.canonical)
            .count();

        let plans = statements
            .iter()
            .enumerate()
            .map(|(index, statement)| {
                let (action, reason) = if index < reused {
                    (Action::Reuse, DecisionReason::ReusableExecution)
                } else if index == diverged_at && diverged_at < session.len() {
                    (Action::Run, DecisionReason::StatementChanged)
                } else {
                    (Action::Run, DecisionReason::NoMatchingExecution)
                };

                StatementPlan {
                    index,
                    range: statement.range,
                    action,
                    reason,
                }
            })
            .collect();

        AlignmentPlan { plans }
    }
}
