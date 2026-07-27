use crate::alignment_plan::{AlignmentPlan, Step};
use crate::decision_reason::{Action, DecisionReason};
use crate::python_source::PythonSource;
use super::SessionHistory;
use super::prefix::prefix_len;

impl SessionHistory {
    /// 이 소스를 실현하려면 무엇을 재사용하고 무엇을 다시 실행해야 하는가.
    ///
    /// 세션을 바꾸지 않으며 실패하지 않는다 — 최악의 답인 전면 Run도 유효한
    /// 계획이기 때문이다.
    ///
    /// 재사용의 근거는 세션의 앞부분이 이 소스의 앞부분과 canonical하게 같다는
    /// 것뿐이다. 같다면 그 실행들은 같은 프로그램을 같은 순서로 같은 시작
    /// 상태에서 실행한 것이므로, 값이 같음을 따로 증명할 필요가 없다 — 문자
    /// 그대로 그 실행이다.
    ///
    /// 세션이 그 prefix 밖에서 무언가를 더 실행했다면 그것이 무엇을 망가뜨렸는지
    /// 알 방법이 아직 없다. 그래서 전부 다시 실행한다. 잘못된 재사용은 조용히
    /// 틀린 결과가 되고 불필요한 재실행은 그냥 느릴 뿐이므로, 모르는 쪽은 언제나
    /// Run이다.
    pub fn align(&self, code: &PythonSource) -> AlignmentPlan {
        let statements = code.statements();
        let common = prefix_len(&self.realized, statements);
        let session_went_further = self.realized.len() > common;
        let disturbed = session_went_further || !self.residue.is_empty();

        let steps = statements
            .iter()
            .enumerate()
            .map(|(index, statement)| {
                let (action, reason) = if index < common {
                    if disturbed {
                        (Action::Run, DecisionReason::DependencyChanged)
                    } else {
                        (Action::Reuse, DecisionReason::ReusableExecution)
                    }
                } else if index == common && session_went_further {
                    (Action::Run, DecisionReason::StatementChanged)
                } else {
                    (Action::Run, DecisionReason::NoMatchingExecution)
                };

                Step {
                    index,
                    range: statement.range,
                    action,
                    reason,
                }
            })
            .collect();

        AlignmentPlan { steps }
    }
}
