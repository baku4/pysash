# Open Issues

지금 당장 고치지 않기로 한 것들. 각 항목은 **무엇이 문제인지 / 왜 지금은 두는지 / 무엇이
바뀌면 다시 보는지**를 적는다. 판정 모델 자체의 결정은 [design.md](design.md)에 있다 — 여기는
그 밖의 것들이다.

## 1. ruff crate 정확 핀 + MSRV가 최신 stable을 따라감

**상태**: open. crates.io 공개의 보류 근거 중 하나.

**무엇이 문제인가**

`Cargo.toml`이 `ruff_python_parser` / `ruff_python_ast` / `ruff_text_size`를 `=0.0.6`으로 정확히
핀하고, `rust-version`이 그 crate들이 요구하는 최신 stable 근처(현재 1.95, 툴체인 1.97.1)를
따라간다. 소비자가 shellwake 하나일 때는 아무 마찰이 없다 — 양쪽을 같은 사람이 통제한다.
불특정 외부 소비자에게 열면 둘 다 마찰이 된다:

- `=` 핀은 소비자의 의존성 트리에서 같은 crate의 다른 버전과 통일되지 않는다. 소비자가 ruff
  crate를 이미 다른 버전으로 쓰고 있으면 두 벌이 들어가고, crates.io 공개 crate에 `=` 핀을
  두는 것은 관례상 기피된다.
- ruff 0.0.x는 공식적으로 "no stability guarantee"이고 MSRV가 짧은 주기로 오른다(3주 만에
  1.94→1.95). 이 crate를 쓰려면 사실상 최신 Rust가 필요하다는 뜻이고, 배포판·기업 환경의
  Rust를 쓰는 소비자는 붙지 못한다.

**왜 지금은 두는가**

핀 자체는 의도된 결정이다 ([design.md §11](design.md#11-ruff를-정확히-핀하는-이유)) — 버전이
섞여 `ruff_python_ast`가 두 벌 들어가는 사고를 막고, ruff의 breaking change가 이 crate의
semver를 인질로 잡지 못하게 한다(ruff 타입이 public API에 없다). 문제는 핀이 아니라 **ruff
crate가 아직 불안정하다는 사실**이고, 그건 우리가 고칠 수 있는 것이 아니다.

**무엇이 바뀌면 다시 보는가**

- ruff crate가 0.1+로 올라가 semver 호환 범위를 약속하면 `=`를 `^`로 완화한다.
- ruff의 MSRV 상승이 진정되면 `rust-version`을 한 단계 뒤로 고정하는 것을 검토한다.
- 그때까지 외부 소비자가 생기면 fallback은 다른 파서가 아니라 같은 ruff crate를 git
  dependency로 받는 것이다 (코드 변경 0).

## 2. 소스 참조가 배열 위치를 씀 — residue 압축이 소스 사본을 못 버림

**상태**: 해결됨. crates.io 공개의 보류 근거였음.

**무엇이 문제인가**

`forget_inert`가 판정에 닿을 수 없는 실행 참조를 버리지만, 그 참조가 가리키던 `PythonSource`
사본은 세션의 `sources` 배열에 그대로 남는다. 실행 참조(`ExecRef`)와 공개 API
(`sources()`, `SessionDiagnostic::OpaqueResidue { source: usize, .. }`)가 소스를 **배열
위치**로 가리키므로, 가운데 소스를 지우면 그 뒤 인덱스가 밀려 참조가 다른 소스를 가리킨다.
소비자가 cell 하나 고칠 때마다 `realize(전체 문서)`를 부르는 편집 루프에서는 소스 사본이
편집 횟수만큼 쌓이고, 세션 메모리가 그에 비례해 자란다.

**어떻게 해결했나**

위치 대신 소유로 가리킨다 — 실행 참조(`ExecRef`)가 `PythonSource`를 직접 든다(전부 `Arc`
백업이라 clone이 O(1)). `sources` 배열이 사라졌고, `forget_inert`가 실행 참조를 버리는 순간
마지막 소유가 풀려 소스가 자동 해제된다. 별도의 정리 단계가 없다.

공개 API 변경: `SessionHistory::sources()` 제거, `SessionDiagnostic::OpaqueResidue`가
`{ source, range }` 대신 `{ text }`로 statement 원문을 직접 싣는다. 회귀 테스트
`forgotten_executions_release_their_sources`가 편집 루프 50회 뒤에도 세션이 붙든 서로 다른
소스 수가 상수(실측 3)임을 검증한다 — 옛 구조에서는 51이었다.
