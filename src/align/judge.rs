use crate::plan;
use crate::plan::{Action, DecisionReason, SessionDiagnostic};
use crate::source::PythonSource;
use crate::statement::Statement;
use super::SessionHistory;
use super::{Hit, hits, residue_entries};
use super::prefix::prefix_len;
use super::self_contained::unresolved_reads;

impl SessionHistory {
    /// Builds a reuse-or-run plan for `code` without changing the session.
    ///
    /// Only undisturbed executions in the common canonical prefix are reusable.
    pub fn align(&self, code: &PythonSource) -> plan::AlignmentPlan {
        let realized: Vec<&Statement> = self
            .realized
            .iter()
            .map(|exec| exec.statement())
            .collect();
        let statements = code.statements();

        let prefix = prefix_len(&realized, statements);
        // Out-of-sequence executions include the divergent suffix and retained residue.
        let outside = || self.realized[prefix..].iter().chain(&self.residue);
        let residue: Vec<(usize, &Statement)> = outside()
            .map(|exec| (exec.seq, exec.statement()))
            .collect();
        let entries = residue_entries(&residue, &self.summaries);

        // Report every retained opaque execution even though one is enough to force all `Run`.
        let diagnostics: Vec<SessionDiagnostic> = outside()
            .filter(|exec| exec.statement().facts.opaque)
            .map(|exec| SessionDiagnostic::OpaqueResidue {
                text: String::from_utf8_lossy(exec.text()).into(),
            })
            .collect();

        let mut unresolved = unresolved_reads(statements);
        let steps: Vec<plan::StatementPlan> = statements
            .iter()
            .enumerate()
            .map(|(index, statement)| {
                let (action, reason) = if self.poisoned {
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
                    // A matching statement at another position is not a valid linear witness.
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
