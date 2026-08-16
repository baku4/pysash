use super::plan::Effect;

/// Conservative facts extracted from one statement for disturbance analysis.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatementFacts {
    /// Names the statement may bind in the module namespace.
    pub binds: Vec<Box<str>>,
    /// Module-level free names the statement reads.
    pub reads: Vec<Box<str>>,
    /// All mentioned names, used to bound possible mutation.
    pub mentions: Vec<Box<str>>,
    /// Alias edges from bare-name assignment and class inheritance.
    pub alias_edges: Vec<(Box<str>, Box<str>)>,
    /// Statically named direct callees.
    pub calls: Vec<Box<str>>,
    /// Callable body summary for a `def` or `class`.
    pub summary: Option<CalleeSummary>,
    /// Names removed by `del`.
    pub deletes: Vec<Box<str>>,
    /// Names whose objects may be mutated in place.
    pub mutates: Vec<Box<str>>,
    pub effect: Effect,
    /// Whether reflective syntax makes the effects unknowable.
    pub opaque: bool,
}

/// An upper bound on the effects of calling a `def` or `class` body.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CalleeSummary {
    /// Module globals the body may bind or delete.
    pub global_writes: Vec<Box<str>>,
    /// Free-name objects the body may mutate in place.
    pub mutates_frees: Vec<Box<str>>,
    /// Positional parameters the body may mutate in place.
    pub mutates_params: Vec<usize>,
    /// Statically named callees used for transitive resolution.
    pub callees: Vec<Box<str>>,
    /// Whether calling the body may execute reflective syntax.
    pub opaque: bool,
}

impl CalleeSummary {
    /// Adds every possible effect from another callable summary.
    pub fn absorb(&mut self, other: &CalleeSummary) {
        for name in &other.global_writes {
            if !self.global_writes.contains(name) {
                self.global_writes.push(name.clone());
            }
        }
        for name in &other.mutates_frees {
            if !self.mutates_frees.contains(name) {
                self.mutates_frees.push(name.clone());
            }
        }
        for position in &other.mutates_params {
            if !self.mutates_params.contains(position) {
                self.mutates_params.push(*position);
            }
        }
        for name in &other.callees {
            if !self.callees.contains(name) {
                self.callees.push(name.clone());
            }
        }
        self.opaque |= other.opaque;
    }
}

impl Default for StatementFacts {
    /// Defaults to opaque so missing analysis can only increase execution.
    fn default() -> Self {
        Self {
            binds: Vec::new(),
            reads: Vec::new(),
            mentions: Vec::new(),
            alias_edges: Vec::new(),
            calls: Vec::new(),
            summary: None,
            deletes: Vec::new(),
            mutates: Vec::new(),
            effect: Effect::Opaque,
            opaque: true,
        }
    }
}
