//! 지금 무엇이 되고 무엇이 안 되는지.
//! `cargo run --example align`

use pysash::Action;
use pysash::python_source::PythonSource;
use pysash::session_history::SessionHistory;

fn main() {
    println!("=== 이어붙이기 — 앞부분을 건너뛴다 ===");
    let mut history = SessionHistory::new();
    for chunk in [
        "import pandas as pd\n",
        "df = pd.read_csv('a.csv')\n",
        "df2 = df.dropna()\n",
    ] {
        history.push(&PythonSource::parse(chunk).unwrap());
    }
    report(
        &history,
        "import pandas as pd\n\
         df = pd.read_csv('a.csv')\n\
         df2 = df.dropna()\n\
         print(df2.shape)\n",
    );

    println!("\n=== 편집 — 전면 재실행으로 떨어진다 ===");
    let mut history = SessionHistory::new();
    for chunk in [
        "import pandas as pd\n",
        "df = pd.read_csv('a.csv')\n",
        "df2 = df.dropna()\nprint(df2.shape)\n",
    ] {
        history.push(&PythonSource::parse(chunk).unwrap());
    }
    let edited = PythonSource::parse(
        "import pandas as pd\n\
         df = pd.read_csv('a.csv')\n\
         df2 = df.dropna().reset_index()\n\
         print(df2.shape)\n",
    )
    .unwrap();
    report_source(&history, &edited);

    println!("\n=== 편집을 realize한 뒤 — 세션이 다시 깨끗해지지 않는다 ===");
    history.realize(&edited);
    report_source(&history, &edited);
}

fn report(history: &SessionHistory, text: &str) {
    report_source(history, &PythonSource::parse(text).unwrap());
}

fn report_source(history: &SessionHistory, code: &PythonSource) {
    let plan = history.align(code);
    println!("{}/{} 재사용", plan.reused_count(), plan.steps.len());

    for step in &plan.steps {
        let text = String::from_utf8_lossy(code.slice(step.range));
        let mark = match step.action {
            Action::Reuse => "reuse",
            Action::Run => "RUN  ",
        };
        println!("  {mark} {:<20?} {}", step.reason, text.trim());
    }
}
