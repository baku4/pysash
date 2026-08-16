//! Aligns Python source with a linear session history to decide which existing statement
//! executions can be reused; PySASH performs static analysis and never executes Python.
//! Incorrect reuse can silently corrupt results while unnecessary execution only costs time,
//! so uncertain cases are always `Run`. See `examples/align.rs` for the complete edit loop.

mod range;
pub use range::Range;

pub mod plan;

mod statement_facts;
mod canonical_statement;
mod statement;

pub mod source;

mod trace;
mod def_use;
mod summaries;

/// A linear record of executions observed in a Python session.
///
/// Sources passed to [`push`](Self::push) or [`realize`](Self::realize) must have completed
/// successfully. Use [`record_partial`](Self::record_partial) when completion is unknown.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SessionHistory {
    realized: Vec<trace::ExecRef>,
    residue: Vec<trace::ExecRef>,
    graph: def_use::DefUseGraph,
    summaries: summaries::SummaryTable,
    // Sequence numbers remain monotonic when `realize` replaces the realized sequence.
    executions: usize,
    poisoned: bool,
}

mod disturbance;
mod forget;

mod record;
mod align;
mod inspect;
