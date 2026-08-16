use super::Range;

/// An ordered execution plan that realizes the input source in the session.
///
/// Executing every `Run` step in order produces a state containing the source's bindings.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AlignmentPlan {
    pub steps: Vec<StatementPlan>,
    /// Length of the longest common canonical prefix.
    pub prefix_len: usize,
    /// Number of retained out-of-prefix executions that can affect this plan.
    pub residue_len: usize,
    /// Session-wide diagnostics for this plan.
    pub diagnostics: Vec<SessionDiagnostic>,
}

/// The decision for one top-level statement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatementPlan {
    /// Zero-based statement position in the input source.
    pub index: usize,
    /// Statement location in the input source bytes.
    pub range: Range,
    pub action: Action,
    pub reason: DecisionReason,
    /// Effect classification for caller policy, not reuse safety.
    pub effect: Effect,
    pub diagnostics: Vec<StatementDiagnostic>,
}

/// Counts derived from an [`AlignmentPlan`] by [`AlignmentPlan::summary`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlanSummary {
    pub total: usize,
    pub reused: usize,
    pub run: usize,
    pub prefix_len: usize,
    pub residue_len: usize,
    /// Index of the first `Run`, or `None` when all steps are reusable.
    pub first_run: Option<usize>,
}

/// Whether to reuse an existing execution or execute the statement again.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Preserve the existing execution without running the statement again.
    Reuse,
    /// Execute the statement again.
    Run,
}

/// Why an [`Action`] was selected.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum DecisionReason {
    /// A matching execution exists and no later execution disturbed its result.
    ReusableExecution,
    /// The session executed a different statement at this position.
    StatementChanged,
    /// No execution at this position can serve as a reuse witness.
    NoMatchingExecution,
    /// A later execution may have mutated an object produced by this statement.
    DependencyChanged { name: Box<str> },
    /// A later execution rebound a name produced by this statement.
    BindingChanged { name: Box<str> },
}

/// A non-fatal diagnostic about the session as a whole.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SessionDiagnostic {
    /// Retained reflective code makes every earlier execution unsafe to reuse.
    OpaqueResidue { text: Box<str> },
}

/// A non-fatal diagnostic attached to one statement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StatementDiagnostic {
    /// The source reads a name that it never binds and that is not a builtin.
    UnresolvedReference { name: Box<str> },
}

/// A statement effect classification for caller policy.
///
/// This value does not participate in the reuse decision.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// No observable effect beyond name binding.
    Pure,
    /// Module import.
    Import,
    /// User-visible output such as printing, display, or logging.
    Output,
    /// Read from files, the network, or user input.
    ExternalRead,
    /// Write to files, the network, or a subprocess.
    ExternalWrite,
    /// Nondeterministic value generation.
    Nondeterministic,
    /// Reflective code with unknown effects.
    Opaque,
}

impl AlignmentPlan {
    /// Derives summary counts from the current steps.
    pub fn summary(&self) -> PlanSummary {
        let run = self
            .steps
            .iter()
            .filter(|step| step.action == Action::Run)
            .count();
        PlanSummary {
            total: self.steps.len(),
            reused: self.steps.len() - run,
            run,
            prefix_len: self.prefix_len,
            residue_len: self.residue_len,
            first_run: self
                .steps
                .iter()
                .find(|step| step.action == Action::Run)
                .map(|step| step.index),
        }
    }

    /// Iterates over `Run` steps in source order.
    pub fn run_steps(&self) -> impl Iterator<Item = &StatementPlan> {
        self.steps.iter().filter(|step| step.action == Action::Run)
    }

    /// Changes every reusable step at or after `index` to `Run`.
    ///
    /// Existing reasons are preserved; this operation never promotes a `Run` to `Reuse`.
    pub fn downgrade_from(&mut self, index: usize) {
        for step in &mut self.steps {
            if step.index >= index && step.action == Action::Reuse {
                step.action = Action::Run;
            }
        }
    }
}
