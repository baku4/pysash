# PySASH
Python Source Alignment with Session History

PySASH aligns Python source with a SessionHistory and determines which statements must be executed again.

```
SessionHistory + Python source
            ↓
      Reuse | Run
```

PySASH performs static analysis only. It does not execute Python code.

## 판정 모델

재사용의 근거는 "값을 다시 계산해도 같다"가 아니라 **"그 실행이 이미 이 소스의 실행이었다"**이다. 세션의 실현 열 앞부분이 소스의 앞부분과 canonical하게 같으면 그 실행들은 같은 프로그램을 같은 순서로 실행한 것이고, 남는 질문은 하나뿐이다 — *그 뒤에 일어난 실행이 그 효과를 훼손했는가?* 이 질문에 실현 밖 실행들(residue)의 오염 상계로 답한다. 애매하면 언제나 Run이다: 잘못된 재사용은 조용히 틀린 결과이고, 불필요한 재실행은 낭비일 뿐이다.

편집 루프는 `align → (Run만 실행) → realize`로 수렴한다. 실행이 중간에 실패해 세션을 믿을 수 없게 되면 `poison()`으로 표시한다 — 이후는 전부 Run이며, 복구는 새 인터프리터 + 새 `SessionHistory`다.

## 이 도구가 서 있는 가정 — 정확히 4개

정적 분석만으로 Python을 건전하게 다루는 것은 불가능하다. 이 도구는 아래 4개의 가정 위에 서 있고, 각각이 깨지는 예를 함께 적는다.

| 가정 | 내용 | 깨지는 예 |
|---|---|---|
| **A-Det** | canonical하게 같은 statement를 같은 순서로 같은 시작 상태에서 실행하면 같은 상태가 된다 | align 사이에 **외부 세계**가 바뀐 경우 — `pd.read_csv('a.csv')`를 재사용했는데 파일이 바뀌었다. 라이브러리는 이걸 볼 수 없으므로 `Effect::ExternalRead`로 표시만 한다. 판단은 호출자의 몫 |
| **A-NoAlias** | 객체는 statement가 구문적으로 언급한 이름, bare-name 대입 별칭(`b = a`), 클래스 상속(`class C(B)`)을 통해서만 변경된다 | 간접 별칭 — `c = [a]` 후 `c[0].append(1)`은 a의 변경으로 잡히지 않는다 |
| **A-NoForeignGlobalWrite** | 세션 밖에 정의된 함수는 이 module의 global을 재바인딩하지 않는다 (Python 의미론상 `global x`는 그 함수 module의 x를 쓴다) | reflection으로 우리 globals를 얻어 쓰는 외부 함수 — 단, 반사 구문이 세션에 보이면 opaque로 떨어져 전부 Run이 된다 |
| **A-KnownEffects** | 화이트리스트 밖 호출의 효과는 결과 바인딩 ∪ 언급된 이름의 in-place 변경 ∪ 세션 정의 callee의 전이 요약 안에 있다 | 고차 함수 — `h = f; h()`처럼 호출 대상이 정적으로 잡히지 않으면 callee의 global 쓰기를 놓칠 수 있다 |

가정이 깨지면 **잘못된 Reuse**(조용히 틀린 결과)가 될 수 있다. 반대 방향 — 정밀도가 부족해 생기는 불필요한 Run — 은 언제나 감수한다.

## 범위 밖 (v0.1)

- IPython magic (`%time`, `!ls`) — 순수 Python만 파싱한다. magic이 있으면 `ParseError`다.
- compound statement 내부의 부분 재사용 — `for` 루프는 통째로 하나다.
- 디스크 영속화, fork/rewind.

## MSRV

ruff 파서 crate를 정확히 핀해서 쓰므로 **MSRV는 최신 stable 근처**를 따라간다. `rust-toolchain.toml`이 정본이다.
