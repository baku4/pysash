use std::collections::HashMap;
use super::disturbance::residue_entries;
use super::statement::Statement;
use super::summaries::SummaryTable;
use super::trace::ExecRef;

/// Removes residue proven unable to affect current or future reuse witnesses.
///
/// Def-use and summary state remain intact. See `docs/design.md` for the proof.
pub fn forget_inert(residue: &mut Vec<ExecRef>, realized: &[ExecRef], summaries: &SummaryTable) {
    // With no realized witness, no residue can affect a future reuse decision.
    let Some(floor) = realized.iter().map(|exec| exec.seq).min() else {
        residue.clear();
        return;
    };

    let outside: Vec<(usize, &Statement)> = residue
        .iter()
        .map(|exec| (exec.seq, exec.statement()))
        .collect();
    let entries = residue_entries(&outside, summaries);

    // The newest identical bound subsumes older ones; unequal ordering is kept conservatively.
    let mut newest: HashMap<(&[String], &[String], bool), usize> = HashMap::new();
    for entry in &entries {
        let bound = (&entry.rebound[..], &entry.mutated[..], entry.opaque);
        newest
            .entry(bound)
            .and_modify(|seq| *seq = (*seq).max(entry.seq))
            .or_insert(entry.seq);
    }

    let live: Vec<bool> = entries
        .iter()
        .map(|entry| {
            let bound = (&entry.rebound[..], &entry.mutated[..], entry.opaque);
            entry.seq > floor && newest[&bound] == entry.seq
        })
        .collect();

    *residue = std::mem::take(residue)
        .into_iter()
        .zip(live)
        .filter_map(|(exec, live)| live.then_some(exec))
        .collect();
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use crate::SessionHistory;
    use crate::plan::Action;
    use crate::source::PythonSource;
    use crate::trace::ExecRef;

    /// A mixed vocabulary that exercises temporal disturbance and compaction.
    const VOCAB: &[&str] = &[
        "x = 1\n",
        "y = x\n",
        "a = []\n",
        "a.append(1)\n",
        "keep = a\n",
        "c = 0\n",
        "def bump():\n    global c\n    c = c + 1\n",
        "def bump():\n    pass\n",
        "bump()\n",
        "import os\n",
        "n = len(a)\n",
    ];

    fn source(picks: &[usize]) -> PythonSource {
        let text: String = picks.iter().map(|pick| VOCAB[*pick]).collect();
        PythonSource::parse(&text).expect("vocab is valid python")
    }

    fn actions(history: &SessionHistory, code: &PythonSource) -> Vec<Action> {
        let plan = history.align(code);
        plan.steps.iter().map(|step| step.action).collect()
    }

    /// Reconstructs source whose prefix covers every realized execution.
    fn mirror(history: &SessionHistory) -> PythonSource {
        let text: String = history
            .realized
            .iter()
            .map(|exec| {
                let text =
                    str::from_utf8(exec.text()).expect("statement boundaries are UTF-8 boundaries");
                format!("{text}\n")
            })
            .collect();
        PythonSource::parse(&text).expect("concatenated statements remain valid Python")
    }

    /// Compaction preserves every verdict across mixed history operations.
    #[test]
    fn forgetting_never_changes_a_verdict() {
        let script = proptest::collection::vec(
            (0..3usize, proptest::collection::vec(0..VOCAB.len(), 0..5)),
            1..8,
        );
        proptest!(|(script in script)| {
            let mut compacted = SessionHistory::new();
            let mut full = SessionHistory::new();
            let mut seen: Vec<PythonSource> = Vec::new();

            for (kind, picks) in &script {
                let code = source(picks);
                seen.push(code.clone());
                let kept = full.residue.clone();
                match kind {
                    0 => {
                        compacted.push(&code);
                        full.push(&code);
                        continue;
                    }
                    1 => {
                        compacted.realize(&code);
                        // Restore the executions displaced by `realize`.
                        let matched = full.align(&code).prefix_len;
                        let displaced: Vec<ExecRef> = full.realized[matched..].to_vec();
                        full.realize(&code);
                        full.residue = kept.into_iter().chain(displaced).collect();
                    }
                    _ => {
                        compacted.record_partial(&code);
                        // Restore the executions added by `record_partial`.
                        let first = full.executions;
                        full.record_partial(&code);
                        let added = (0..code.statements().len()).map(|index| ExecRef {
                            source: code.clone(),
                            index,
                            seq: first + index,
                        });
                        full.residue = kept.into_iter().chain(added).collect();
                    }
                }
            }

            seen.push(mirror(&compacted));
            for code in &seen {
                prop_assert_eq!(actions(&compacted, code), actions(&full, code));
            }
            prop_assert!(compacted.residue_count() <= full.residue_count());
        });
    }

    /// An empty realized sequence makes every residue entry unreachable.
    #[test]
    fn an_empty_realized_column_leaves_nothing_reachable() {
        let mut history = SessionHistory::new();
        history.record_partial(&PythonSource::parse("x = 1\n").expect("valid python"));
        assert_eq!(history.residue_count(), 0);
    }

    /// Forgotten executions release their source storage.
    #[test]
    fn forgotten_executions_release_their_sources() {
        let mut history = SessionHistory::new();
        history.push(&PythonSource::parse("import os\nbase = 1\n").unwrap());

        let held_sources = |history: &SessionHistory| {
            let mut raws: Vec<*const u8> = history
                .realized
                .iter()
                .chain(&history.residue)
                .map(|exec| exec.source.raw().as_ptr())
                .collect();
            raws.sort_unstable();
            raws.dedup();
            raws.len()
        };

        let mut peak = 0;
        for i in 0..50 {
            let code = PythonSource::parse(&format!("import os\nbase = 1\nx = {i}\n")).unwrap();
            history.realize(&code);
            peak = peak.max(held_sources(&history));
        }
        assert!(peak <= 4, "retained sources grew with the edit count: {peak}");
    }

}
