//! An end-to-end edit loop for `SessionHistory`.

use pysash::SessionHistory;
use pysash::plan::{Action, AlignmentPlan};
use pysash::source::{ParseError, PythonSource};

fn main() -> Result<(), ParseError> {
    // Parse a source prefix that has already completed successfully in Python.
    let prefix = PythonSource::parse("import math\nradius = 2.0\n")?;

    // Push that successful execution into a new linear session history.
    let mut history = SessionHistory::new();
    history.push(&prefix);

    // Align the complete script and reuse the prefix already in the session.
    let script = PythonSource::parse(SCRIPT)?;
    let plan = history.align(&script);
    assert_eq!(
        actions(&plan),
        [Action::Reuse, Action::Reuse, Action::Run, Action::Run]
    );

    // Run each `Run` step in source order through the caller's Python runtime.
    run_steps(&plan, &script);

    // Realize the source only after every requested execution succeeds.
    history.realize(&script);

    // Aligning the realized source again converges to complete reuse.
    assert!(history.align(&script).run_steps().next().is_none());

    // Edit above the old run frontier while retaining the safe import prefix.
    let edited = PythonSource::parse(EDITED)?;
    let plan = history.align(&edited);
    assert_eq!(
        actions(&plan),
        [Action::Reuse, Action::Run, Action::Run, Action::Run]
    );
    run_steps(&plan, &edited);
    history.realize(&edited);

    // Record known source after an interrupted execution without poisoning unrelated reuse.
    let interrupted = PythonSource::parse("radius = 99.0\nraise RuntimeError('stopped')\n")?;
    history.record_partial(&interrupted);
    assert_eq!(
        actions(&history.align(&edited)),
        [Action::Reuse, Action::Run, Action::Reuse, Action::Reuse]
    );

    // Poison the history only when unknown code may have changed the session.
    history.poison();
    assert!(
        history
            .align(&edited)
            .steps
            .iter()
            .all(|step| step.action == Action::Run)
    );

    Ok(())
}

const SCRIPT: &str = "import math\n\
                      radius = 2.0\n\
                      area = math.pi * radius ** 2\n\
                      print(area)\n";

const EDITED: &str = "import math\n\
                      radius = 3.0\n\
                      area = math.pi * radius ** 2\n\
                      print(area)\n";

fn actions(plan: &AlignmentPlan) -> Vec<Action> {
    plan.steps.iter().map(|step| step.action).collect()
}

fn run_steps(plan: &AlignmentPlan, source: &PythonSource) {
    for step in plan.run_steps() {
        let statement = String::from_utf8_lossy(source.slice(step.range));
        println!("python> {}", statement.trim());
    }
}
