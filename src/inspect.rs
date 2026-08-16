use super::SessionHistory;

impl SessionHistory {
    /// Returns the number of realized statement executions.
    pub fn statement_count(&self) -> usize {
        self.realized.len()
    }

    /// Returns the retained out-of-sequence executions that can still affect alignment.
    pub fn residue_count(&self) -> usize {
        self.residue.len()
    }

    /// Iterates over names currently recorded as bound.
    pub fn live_names(&self) -> impl Iterator<Item = &str> {
        self.graph.live_names()
    }
}
