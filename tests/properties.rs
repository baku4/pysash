//! Invariants over generated session-and-source pairs.

use proptest::prelude::*;
use pysash::source::PythonSource;
use pysash::SessionHistory;
use pysash::plan::Action;

/// A mixed vocabulary covering binding, mutation, aliases, and transitive writes.
const VOCAB: &[&str] = &[
    "x = 1\n",
    "y = x\n",
    "a = []\n",
    "a.append(1)\n",
    "keep = a\n",
    "c = 0\n",
    "def bump():\n    global c\n    c = c + 1\n",
    "bump()\n",
    "import os\n",
    "n = len(a)\n",
];

fn statements() -> impl Strategy<Value = Vec<&'static str>> {
    proptest::collection::vec(0..VOCAB.len(), 0..8)
        .prop_map(|picks| picks.into_iter().map(|i| VOCAB[i]).collect())
}

fn session(pushed: &[&str]) -> SessionHistory {
    let mut history = SessionHistory::new();
    for text in pushed {
        history.push(&PythonSource::parse(text).expect("vocab is valid python"));
    }
    history
}

proptest! {
    /// Reuse never appears outside the common prefix.
    #[test]
    fn reuse_stays_inside_the_prefix(pushed in statements(), incoming in statements()) {
        let history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        let plan = history.align(&code);

        prop_assert_eq!(plan.summary().reused + plan.summary().run, plan.summary().total);
        for step in plan.steps.iter().filter(|step| step.action == Action::Reuse) {
            prop_assert!(step.index < plan.prefix_len);
        }
    }

    /// Realizing any generated plan converges immediately.
    #[test]
    fn realize_converges_in_one_step(pushed in statements(), incoming in statements()) {
        let mut history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        history.realize(&code);
        let plan = history.align(&code);
        prop_assert!(
            plan.run_steps().next().is_none(),
            "not converged: {:?}",
            plan.steps.iter().map(|p| p.action).collect::<Vec<_>>()
        );
    }

    /// Alignment is pure and repeatable.
    #[test]
    fn align_is_pure(pushed in statements(), incoming in statements()) {
        let history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        prop_assert_eq!(history.align(&code), history.align(&code));
    }

    /// Recording a partial execution can only reduce reuse.
    #[test]
    fn recording_an_interruption_never_adds_reuse(
        pushed in statements(),
        interrupted in statements(),
        incoming in statements(),
    ) {
        let mut history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        let before = history.align(&code).summary().reused;

        history.record_partial(&PythonSource::parse(&interrupted.concat()).expect("valid"));
        prop_assert!(history.align(&code).summary().reused <= before);
    }

    /// A partial execution still converges after one realization.
    #[test]
    fn realize_converges_after_an_interruption(
        pushed in statements(),
        interrupted in statements(),
        incoming in statements(),
    ) {
        let mut history = session(&pushed);
        history.record_partial(&PythonSource::parse(&interrupted.concat()).expect("valid"));
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        history.realize(&code);
        let plan = history.align(&code);
        prop_assert!(
            plan.run_steps().next().is_none(),
            "not converged: {:?}",
            plan.steps.iter().map(|p| p.action).collect::<Vec<_>>()
        );
    }
}
