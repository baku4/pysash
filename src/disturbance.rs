use super::def_use::DefUseGraph;
use super::statement::Statement;
use super::summaries::SummaryTable;

/// An upper bound on what one out-of-sequence execution may have disturbed.
pub struct ResidueEntry {
    /// Global order; this entry can disturb only earlier executions.
    pub seq: usize,
    /// Names possibly rebound or deleted, including transitive global writes.
    pub rebound: Vec<String>,
    /// Names whose objects may have been mutated in place.
    pub mutated: Vec<String>,
    /// Whether the execution may have disturbed any earlier state.
    pub opaque: bool,
}

/// How a later disturbance intersects a statement's produced state.
pub enum Hit {
    /// A produced name was rebound.
    Rebound(String),
    /// A produced object may have been mutated.
    Mutated(String),
    /// A later execution has unknown effects.
    Opaque,
}

/// Computes the disturbance upper bound for each residue execution.
pub fn residue_entries(
    residue: &[(usize, &Statement)],
    summaries: &SummaryTable,
) -> Vec<ResidueEntry> {
    residue
        .iter()
        .map(|(seq, statement)| {
            let facts = &statement.facts;
            let mut entry = ResidueEntry {
                seq: *seq,
                rebound: Vec::new(),
                mutated: Vec::new(),
                opaque: facts.opaque,
            };
            for name in facts.binds.iter().chain(&facts.deletes) {
                push_unique(&mut entry.rebound, name);
            }
            for name in &facts.mutates {
                push_unique(&mut entry.mutated, name);
            }
            for call in &facts.calls {
                // Resolve the definition that was live when this execution occurred.
                if let Some(summary) = summaries.resolve(call, *seq) {
                    entry.opaque |= summary.opaque;
                    for name in &summary.global_writes {
                        push_unique(&mut entry.rebound, name);
                    }
                    for name in &summary.mutates_frees {
                        push_unique(&mut entry.mutated, name);
                    }
                }
            }
            entry
        })
        .collect()
}

/// Returns the first later residue entry that may disturb this execution's result.
pub fn hits(
    entries: &[ResidueEntry],
    seq: usize,
    statement: &Statement,
    summaries: &SummaryTable,
    graph: &DefUseGraph,
) -> Option<Hit> {
    let facts = &statement.facts;

    // Include names bound by the statement and objects it may create or mutate.
    let mut produces: Vec<String> = Vec::new();
    for name in facts
        .binds
        .iter()
        .chain(&facts.deletes)
        .chain(&facts.mutates)
    {
        push_unique(&mut produces, name);
    }
    for call in &facts.calls {
        // Resolve the definition that was live when this execution occurred.
        if let Some(summary) = summaries.resolve(call, seq) {
            for name in summary.global_writes.iter().chain(&summary.mutates_frees) {
                push_unique(&mut produces, name);
            }
        }
    }

    for entry in entries.iter().filter(|entry| entry.seq > seq) {
        if entry.opaque {
            return Some(Hit::Opaque);
        }
        for name in &produces {
            if entry.rebound.contains(name) {
                return Some(Hit::Rebound(name.clone()));
            }
        }
        // Only mutation follows aliases, using edges that existed at the disturbance time.
        let mut reachable = produces.clone();
        graph.alias_closure(&mut reachable, entry.seq);
        for name in &reachable {
            if entry.mutated.contains(name) {
                return Some(Hit::Mutated(name.clone()));
            }
        }
    }
    None
}

fn push_unique(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|existing| existing == name) {
        names.push(name.to_string());
    }
}
