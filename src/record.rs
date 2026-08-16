use super::forget::forget_inert;
use super::source::PythonSource;
use super::trace::ExecRef;
use super::SessionHistory;
use super::plan::Action;

impl SessionHistory {
    /// Creates an empty, trusted session history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends every statement from a successfully executed source.
    ///
    /// The caller must not pass unexecuted or partially executed source.
    pub fn push(&mut self, code: &PythonSource) {
        for index in 0..code.statements().len() {
            let exec = self.record_execution(code, index);
            self.realized.push(exec);
        }
    }

    /// Records completion of this source's alignment plan and makes it the realized sequence.
    ///
    /// The caller must execute every `Run` step in source order before calling this method.
    pub fn realize(&mut self, code: &PythonSource) {
        let plan = self.align(code);
        let matched = plan.prefix_len;

        let displaced: Vec<ExecRef> = self.realized.drain(matched..).collect();
        self.residue.extend(displaced);

        // Re-executed prefix entries need new sequence numbers so old residue cannot affect them.
        for step in plan.steps.iter().take(matched) {
            if step.action == Action::Run {
                let exec = self.record_execution(code, step.index);
                self.realized[step.index] = exec;
            }
        }
        for index in matched..code.statements().len() {
            let exec = self.record_execution(code, index);
            self.realized.push(exec);
        }
        forget_inert(&mut self.residue, &self.realized, &self.summaries);
    }

    /// Records every statement as a possible effect of an incomplete execution.
    ///
    /// Record any known completed prefix first so this source receives later sequence numbers.
    pub fn record_partial(&mut self, code: &PythonSource) {
        for index in 0..code.statements().len() {
            let exec = self.record_execution(code, index);
            self.residue.push(exec);
        }
        forget_inert(&mut self.residue, &self.realized, &self.summaries);
    }

    /// Marks the session state as unknowable, forcing all subsequent plans to `Run`.
    ///
    /// Recovery requires a new interpreter and a new `SessionHistory`.
    pub fn poison(&mut self) {
        self.poisoned = true;
    }

    /// Allocates a sequence number and records all time-indexed analysis together.
    fn record_execution(&mut self, source: &PythonSource, index: usize) -> ExecRef {
        let seq = self.executions;
        self.executions += 1;
        let facts = &source.statements()[index].facts;
        self.graph.record(facts, seq);
        self.summaries.record(facts, seq);
        ExecRef {
            source: source.clone(),
            index,
            seq,
        }
    }
}
