//! 무작위 (세션, 소스) 쌍에 대해 절대 깨지면 안 되는 성질들.

use proptest::prelude::*;
use pysash::python_source::PythonSource;
use pysash::{Action, SessionHistory};

/// 실제 세션에 나올 법한 statement들. 바인딩·mutation·전이 global 쓰기·별칭이
/// 골고루 섞여 있어야 오염 계산의 성질이 실제로 시험된다.
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
    /// Reuse는 prefix 밖에서 절대 나오지 않고, witness는 Reuse에만 붙는다.
    #[test]
    fn reuse_stays_inside_the_prefix(pushed in statements(), incoming in statements()) {
        let history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        let plan = history.align(&code);

        prop_assert_eq!(plan.summary.reused + plan.summary.run, plan.summary.total);
        for statement in &plan.plans {
            match statement.action {
                Action::Reuse => {
                    prop_assert!(statement.index < plan.summary.prefix_len);
                    prop_assert_eq!(statement.witness, Some(statement.index));
                }
                Action::Run => prop_assert_eq!(statement.witness, None),
            }
        }
    }

    /// 어떤 세션에서든, 계획을 실행하고 realize하면 그 자리에서 수렴한다.
    #[test]
    fn realize_converges_in_one_step(pushed in statements(), incoming in statements()) {
        let mut history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        history.realize(&code);
        let plan = history.align(&code);
        prop_assert!(
            plan.run_plans().next().is_none(),
            "not converged: {:?}",
            plan.plans.iter().map(|p| p.action).collect::<Vec<_>>()
        );
    }

    /// align은 순수하다 — 같은 세션에 두 번 물어도 같은 답이다.
    #[test]
    fn align_is_pure(pushed in statements(), incoming in statements()) {
        let history = session(&pushed);
        let code = PythonSource::parse(&incoming.concat()).expect("valid");
        prop_assert_eq!(history.align(&code), history.align(&code));
    }
}
