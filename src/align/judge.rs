use crate::plan;
use crate::plan::{Action, DecisionReason, SessionDiagnostic};
use crate::source::PythonSource;
use crate::statement::Statement;
use super::SessionHistory;
use super::{Hit, hits, residue_entries};
use super::prefix::prefix_len;
use super::self_contained::unresolved_reads;

impl SessionHistory {
    /// 이 소스를 실행하려면 무엇을 재사용하고 무엇을 다시 실행해야 하는가.
    ///
    /// 세션을 바꾸지 않으며 실패하지 않는다 — 전부 다시 실행하라는 것도 유효한
    /// 계획이기 때문이다.
    ///
    /// 재사용의 근거는 하나뿐이다: **실현 열의 앞이 이 소스의 앞과 같고, 그 실행이
    /// 남긴 것을 그 뒤의 어떤 실행도 건드리지 않았다.** 앞 조건은 canonical 비교가,
    /// 뒤 조건은 실현 밖 실행들(residue)의 오염 상계가 판정한다. 애매한 것은 전부
    /// Run으로 떨어진다.
    pub fn align(&self, code: &PythonSource) -> plan::AlignmentPlan {
        let realized: Vec<&Statement> = self
            .realized
            .iter()
            .map(|exec| exec.statement(&self.sources))
            .collect();
        let statements = code.statements();

        let prefix = prefix_len(&realized, statements);
        // 실현 밖 실행 = 이 소스와 갈라진 뒤의 실현 열 + 이전에 밀려난 것들.
        let outside = || self.realized[prefix..].iter().chain(&self.residue);
        let residue: Vec<(usize, &Statement)> = outside()
            .map(|exec| (exec.seq, exec.statement(&self.sources)))
            .collect();
        let entries = residue_entries(&residue, &self.summaries);

        // 반사적 구문은 하나만 있어도 오염이 전체가 되지만, 전부 싣는다 — 무엇이
        // 세션을 알 수 없게 만들었는지는 호출자가 봐야 한다. 갈라졌다는 사실
        // 자체는 summary.residue_len이 이미 말한다.
        let diagnostics: Vec<SessionDiagnostic> = outside()
            .filter(|exec| exec.statement(&self.sources).facts.opaque)
            .map(|exec| SessionDiagnostic::OpaqueResidue {
                source: exec.source,
                range: exec.statement(&self.sources).range,
            })
            .collect();

        let mut unresolved = unresolved_reads(statements);
        let steps: Vec<plan::StatementPlan> = statements
            .iter()
            .enumerate()
            .map(|(index, statement)| {
                let (action, reason) = if self.poisoned {
                    // 세션 상태 자체를 믿을 수 없다 — 근거로 삼을 실행이 없다.
                    (Action::Run, DecisionReason::NoMatchingExecution)
                } else if index < prefix {
                    let seq = self.realized[index].seq;
                    match hits(&entries, seq, statement, &self.summaries, &self.graph) {
                        None => (Action::Reuse, DecisionReason::ReusableExecution),
                        Some(Hit::Rebound(name)) => (
                            Action::Run,
                            DecisionReason::BindingChanged { name: name.into() },
                        ),
                        Some(Hit::Mutated(name)) => (
                            Action::Run,
                            DecisionReason::DependencyChanged { name: name.into() },
                        ),
                        Some(Hit::Opaque) => (Action::Run, DecisionReason::NoMatchingExecution),
                    }
                } else if index == prefix && prefix < realized.len() {
                    (Action::Run, DecisionReason::StatementChanged)
                } else if let Some(read) = realized
                    .iter()
                    .any(|past| past.canonical == statement.canonical)
                    .then(|| statement.facts.reads.first())
                    .flatten()
                {
                    // 같은 문장을 실행한 적은 있지만 이 자리의 실행이 아니다 —
                    // 문맥이 다르므로 다시 실행한다.
                    (
                        Action::Run,
                        DecisionReason::DependencyChanged { name: read.clone() },
                    )
                } else {
                    (Action::Run, DecisionReason::NoMatchingExecution)
                };

                plan::StatementPlan {
                    index,
                    range: statement.range,
                    effect: statement.facts.effect,
                    action,
                    reason,
                    diagnostics: std::mem::take(&mut unresolved[index]),
                }
            })
            .collect();

        plan::AlignmentPlan {
            steps,
            prefix_len: prefix,
            residue_len: residue.len(),
            diagnostics,
        }
    }
}
