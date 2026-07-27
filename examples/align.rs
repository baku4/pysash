//! `cargo run --example align`

use pysash::Action;
use pysash::python_source::PythonSource;
use pysash::session_history::SessionHistory;

fn main() {
    println!("=== 이어붙이기 ===");
    let mut history = SessionHistory::new();
    for chunk in [
        "import pandas as pd\n",
        "df = pd.read_csv('a.csv')\n",
        "df2 = df.dropna()\n",
    ] {
        history.push(&PythonSource::parse(chunk).unwrap());
    }
    report(&history, &parse(SCRIPT));

    println!("\n=== REPL에서 이것저것 해보다가 스크립트를 붙여넣기 ===");
    let mut history = SessionHistory::new();
    history.push(&PythonSource::parse("tmp = 1\ndel tmp\n").unwrap());
    for chunk in ["import pandas as pd\n", "df = pd.read_csv('a.csv')\n"] {
        history.push(&PythonSource::parse(chunk).unwrap());
    }
    report(&history, &parse(SCRIPT));

    println!("\n=== 가운데 줄 편집 — 1회차 ===");
    let mut history = SessionHistory::new();
    for chunk in [
        "import pandas as pd\n",
        "df = pd.read_csv('a.csv')\n",
        "df2 = df.dropna()\nprint(df2.shape)\n",
    ] {
        history.push(&PythonSource::parse(chunk).unwrap());
    }
    let edited = parse(EDITED);
    report(&history, &edited);

    println!("\n=== 실행한 것을 기록한 뒤 — 2회차 ===");
    history.push(&edited);
    report(&history, &edited);
}

const SCRIPT: &str = "import pandas as pd\n\
                      df = pd.read_csv('a.csv')\n\
                      df2 = df.dropna()\n\
                      print(df2.shape)\n";

const EDITED: &str = "import pandas as pd\n\
                      df = pd.read_csv('a.csv')\n\
                      df2 = df.dropna().reset_index()\n\
                      print(df2.shape)\n";

fn parse(text: &str) -> PythonSource {
    PythonSource::parse(text).unwrap()
}

fn report(history: &SessionHistory, code: &PythonSource) {
    let plan = history.align(code);
    let reused = plan.plans.len() - plan.run_plans().count();
    println!("{}/{} 재사용", reused, plan.plans.len());

    for entry in &plan.plans {
        let text = String::from_utf8_lossy(code.slice(entry.range));
        let mark = match entry.action {
            Action::Reuse => "reuse",
            Action::Run => "RUN  ",
        };
        println!("  {mark} {:<20?} {}", entry.reason, text.trim());
    }
}
