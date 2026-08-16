# 판정 모델 — 왜 이렇게 판단하는가

`README.md`가 이 도구가 **무엇을** 하는지를 말한다면, 이 문서는 **왜 그렇게 판단하는지**를
말한다. 코드를 읽어서는 알 수 없는 것들 — 어떤 대안을 왜 버렸는지, 상계를 어디서
끊었는지, 어떤 가정 위에 서 있는지 — 만 담는다.

타입 하나하나의 계약은 여기 없다. 그건 rustdoc이 정본이다 (`cargo doc --open`).

---

## 1. 비대칭 — 모든 결정의 뿌리

**잘못된 Reuse는 조용히 틀린 결과다. 불필요한 Run은 그냥 낭비다.**

이 비대칭이 이 crate 전체의 판정 기준이다. 그래서:

- **all-Run은 언제나 유효한 계획이다.** 모든 Reuse는 그 안전지대에서의 이탈이고 증명 의무를 진다.
- 이 비대칭을 주석이나 장식용 타입이 아니라 **제어 흐름의 모양**으로 표현한다.
  `Action::Reuse`를 만드는 표현식은 판정 함수 안에 **정확히 하나뿐이고**, "공통 prefix 안"
  분기에 있다. 새 케이스를 추가하는 사람이 실수로 "일단 Reuse"를 쓸 자리가 없다.
- 애매하면 무조건 Run이다. 근사가 틀리는 방향은 언제나 Run이 늘어나는 쪽이어야 한다.

---

## 2. 재사용의 근거 — 실현 의미론

### 2.1 무엇을 재사용한다는 것인가

재사용은 **"값을 다시 계산해도 같다"가 아니라 "그 실행이 이미 이 소스의 실행이었다"**이다.

> **정의 (Realization).** 계획 `d`와 witness map `φ`(Reuse step ↦ 세션 내 위치)가 소스 `P`를
> *실현한다*는 것은, 실행 열
> `⟨ exec(h_{φ(j)}) if d(j)=Reuse ; exec(p_j) if d(j)=Run ⟩` (j = 1..m)
> 이 `P`의 유효한 실행이라는 것이다 — 즉 재사용된 각 실행이 `P`가 그 지점에서 놓였을 상태와
> 관측 동등한 상태에서 일어났고, 그 실행의 효과를 이후 다른 실행이 훼손하지 않았다.

> **정의 (Super-set).** 계획이 **정확(correct)**하다는 것은:
> ```
> ∀ n ∈ binds(P).   σ_plan(n) ≅ σ_real(n)     -- 값에 대해서는 등식
> dom(σ_plan)       ⊇ dom(σ_real)              -- 이름 집합은 이상(以上)
> ```
> `σ_real`은 위 실현 열이 만드는 상태다. `≅`는 도달 가능한 객체 그래프 위의 관측 동등성.
> **이름 집합이 "이상"인 것 — 실현 밖 실행이 남긴 잉여 바인딩이 세션에 남아 있어도 무방하다는
> 것 — 이 "super-set"의 뜻이다.**

### 2.2 왜 이 정의를 골랐나

대안은 **append 의미론**이었다 — "지금 세션 위에 `P`를 통째로 얹은 상태"를 기준으로 삼는 것.
그쪽은 all-Run이 자명하게 정답이라 안전지대가 공짜지만, **동일 소스를 다시 정렬해도
`acc = acc + 1` 류는 반드시 Run이어야 한다.** 그게 정답이기 때문이다. 재사용할 수 있는 것이
"그 자리에서 멱등인 statement"뿐이 되어 헤드라인 기능이 부정확해지는 게 아니라 **불가능해진다.**

실현 의미론에서는 그 대신 이런 성질들이 따라온다:

- 동일 소스 재정렬 = 100% 재사용.
- **비결정성이 건전성 문제에서 사라진다.** `x = random()`을 재사용해도 값은 그것이 맞다 —
  그 실행이 `P`의 실행이기 때문이다. `print`도 이미 제때 출력됐다. 그래서 effect 기반 정책
  기계가 통째로 필요 없다.
- 대가: **all-Run이 자동으로 정답이 아니다.** 오염된 세션 위에서 `P` 전체를 돌리면
  `acc = acc + 1`이 이중 계산된다. 완전한 fallback은 "all-Run"이 아니라 "세션 리셋 후 all-Run"이다.

### 2.3 왜 prefix인가

이전 설계들은 전부 재사용을 `fingerprint(canonical, epoch, {의존 바인딩의 버전})` 일치로
판정하려 했다. 그건 **값 동등성을 def-use 그래프 위에서 재유도**하려는 시도이고, 재유도가
성립하려면 statement의 입출력을 정확히 알아야 한다. 그런데 Python에서는 `add(a)` 한 줄의
출력 집합조차 정적으로 알 수 없다. 전부 여기서 뚫렸다.

**Prefix는 재유도를 하지 않는다.** `p₁..p_r`이 세션 실행 열의 `h₁..h_r`과 canonical하게 같으면,
그 실행들은 *같은 프로그램을, 같은 순서로, 같은 시작 상태에서* 실행한 것이다. 값이 같음을
증명할 필요가 없다 — **문자 그대로 그 실행이다.** 남는 질문은 하나뿐이다:
*그 뒤에 세션이 추가로 한 일이 그 효과를 훼손했는가?*

이 전환으로 사라진 것들: Merkle fingerprint, epoch/opaque chain, heap-SSA region, 2-pass
fixpoint, nonce rule, 7단계 effect lattice, policy trait, assumption set.

---

## 3. 알고리즘

전체가 **O(n + m)**이다. 재귀도, fixpoint 재수렴 루프도 없다.

### 3.1 Step 1 — prefix match (위치로 고정)

```
r = 0
while r < realized.len() && r < code.statements().len()
   && realized[r].canonical == code.statements()[r].canonical:
    r += 1
```

**위치로 고정한다. 전역 content-addressed 조회를 하지 않는다.** 이것이 multiplicity 버그
부류를 구조적으로 제거한다 — `add(a)`가 소스에 두 번 나오면 두 개의 다른 위치이지 같은
witness를 두 번 소비하는 것이 아니다.

**동시에 이것이 세션의 linear 제약을 그대로 구현한다.** `H=[a;b]`, `P=[b;a]`는 `r=0`이다 —
순서가 바뀌면 재사용이 없다. jupyter처럼 위아래를 오가는 모델도, marimo처럼 선언 순서가
무관한 모델도 표현 불가능해진다. 그게 의도다.

### 3.2 Step 2 — 오염 집합

실현 밖 실행(residue) = `realized[r..]` + 이전에 밀려난 것 중 세션이 들고 있는 것(§3.7). 이들이
세션 상태에서 망가뜨렸을 수 있는 것의 상계를 잡는다.

각 residue 실행 하나에 대해:

```
rebound = binds ∪ deletes ∪ ⋃ { summary*(c).global_writes : c ∈ calls }
mutated = mutates ∪ ⋃ { summary*(c).mutates_frees : c ∈ calls }
opaque  = facts.opaque ∨ ⋁ { summary*(c).opaque : c ∈ calls }
```

`poisoned`거나 residue에 opaque가 하나라도 있으면 그 뒤는 전부 Run이다. **safe degrade의
유일한 출구이고, 여기로 떨어지는 것은 버그가 아니라 설계다.**

### 3.3 Step 3 — 판정

각 statement `p_j`에 대해:

```
if poisoned:                      (Run, NoMatchingExecution)
elif j < r:                       -- 공통 prefix 안
    match 오염이 produces(p_j)를 건드렸는가:
        아니오        => (Reuse, ReusableExecution)     ← Reuse를 만드는 유일한 자리
        재바인딩(n)   => (Run, BindingChanged { n })
        변경(n)       => (Run, DependencyChanged { n })
        알 수 없음    => (Run, NoMatchingExecution)
elif j == r && r < realized.len():  (Run, StatementChanged)      -- 편집 지점
elif 세션이 이 문장을 어디선가 실행한 적이 있다:
                                    (Run, DependencyChanged { 첫 read })  -- 문맥이 다르다
else:                               (Run, NoMatchingExecution)
```

```
produces(s) = binds(s) ∪ deletes(s)
            ∪ ⋃ { summary*(c).global_writes ∪ summary*(c).mutates_frees : c ∈ calls(s) }
            ∪ mutates(s)
```

오염과 `produces`를 맞대볼 때, **재바인딩은 이름 그대로 비교하고 in-place 변경은 별칭 폐포까지
넓혀서 비교한다.** 이름을 다시 묶는 것은 별칭이 가리키던 객체를 건드리지 않기 때문이다.

**다섯 개 `DecisionReason`이 상호배타적이고 완전한 결정 트리를 이룬다.**

### 3.4 실행 순서가 오염 계산에 들어가는 이유

오염을 "시간을 무시한 이름 집합"으로 다루면 **편집 루프가 영원히 수렴하지 않는다.**
`realize`가 옛 실행을 residue로 밀어낸 뒤 다시 정렬하면, 옛 실행이 **자기보다 나중에 일어난**
새 실행을 오염시킨 것으로 계산되기 때문이다.

§2.1의 실현 정의("그 실행의 효과를 **이후** 다른 실행이 훼손하지 않았다")가 정본이므로,
모든 실행에 전역 순번(`seq`)을 붙이고 **판정 대상보다 뒤에 일어난 residue 실행만** 오염
후보로 잡는다. 순번은 `realize`가 실현 열을 교체해도 절대 되돌아가지 않는다.

같은 이유로 두 곳이 더 시간을 안다. 무작위 세션에 대한 property test가 잡아낸 반례들이다.

- **함수 요약.** 나중의 `def f` 재정의를 합집합해 버리면 그보다 먼저 일어난 `f()` 호출의
  `produces`가 소급 팽창한다. 그래서 요약은 `(정의 순번, 요약)`으로 쌓고, 호출 시점보다 앞선
  최신 정의로 해석한다 — Python의 late binding과도 일치한다.
- **별칭 폐포.** 나중에 생긴 `p = a` 별칭이 그 전에 일어난 변경을 소급 전파한다. 그래서 폐포는
  "그 변경 시점까지 존재한 간선"으로만 계산한다. union-find는 시간을 자를 수 없어 간선 리스트
  + 시점 필터로 대체했다.

### 3.5 세션 갱신 — `push` / `realize` / `record_partial`

- **순수 append 워크플로(REPL)**: `push`. 실현 열 뒤에 그대로 잇는다. residue가 비어 있으므로
  다음 정렬의 prefix는 항상 최대다.
- **계획 실행 후**: `realize`. 실현 열을 이 소스로 교체하고, 밀려난 옛 실행들을 residue로 옮긴다.
- **실행이 중간에 끊겼을 때**: `record_partial`. 소스를 통째로 residue에 넣는다 — §3.6.

`realize`는 **계획을 인자로 받지 않는다.** 내부에서 같은 판정을 다시 계산하므로 위조된 계획이
세션을 오염시키는 경로가 아예 없다. 이전 설계들이 "plan 위조 방어"에 타입 예산을 썼는데,
공격면 자체를 API에서 제거하는 편이 싸고 확실하다.

`realize`에는 한 단계가 더 있다. **공통 prefix 안에서 오염 때문에 Run이 된 statement는 방금
다시 실행된 것이므로, 그 자리를 새 순번의 실행으로 바꿔 단다.** 옛 실행을 그대로 두면 이미
지나간 오염이 영원히 그 자리를 Run으로 만든다.

새 순번은 **def-use 그래프와 요약 표에도 함께 기록한다.** canonical이 같으니 더할 정보가 없어
보이지만, §3.4의 시점 해석이 그 직관을 깬다 — 요약 조회는 "호출 시점보다 앞선 **최신** 정의"를
쓰므로, 그 사이에 같은 이름의 재정의가 끼어들었다면 다시 실행된 정의가 그것을 덮어야 한다.
안 덮으면 이 실행보다 뒤에 놓인 호출의 `produces`가 낡은 정의로 계산되어 **좁아지고**, 좁아진
상계는 잘못된 재사용이다. 그래서 실행을 만드는 자리는 코드에 하나뿐이다 — 순번 발급과
그래프·요약 기록이 갈라질 수 없게 묶어 둔다.

### 3.6 어디까지 돌았는지 모르는 실행

노트북 개발 루프에서 실행 에러는 예외가 아니라 **일상**이다. 그런데 예외로 멈춘 순간
인터프리터에는 **어디까지인지 모르는** 부분 효과가 남는다 — IPython은 cell 소스를 통째로 받아
돌리고 실패한 statement의 인덱스를 주지 않으며, traceback의 줄 번호는 중첩 호출과 `exec` 때문에
믿을 수 없다. 취소와 커널 사망도 같은 모양이다.

두 극단이 다 틀렸다.

- **`poison()`으로 처리한다** — 세션 하나를 통째로 버린다. 오타 한 번에 맨 위의 5 GB CSV 로딩까지
  영구히 재사용 불가가 되므로, 이 라이브러리를 쓰는 이유 자체가 사라진다.
- **완주한 prefix만 `realize`하고 끝낸다** — 조용히 틀린다. `x = 1` / `x = 2; boom()`에서 세션은
  x를 1이라고 믿지만 인터프리터의 x는 2일 수 있다.

우리가 모르는 것은 "무슨 일이 있었는가"가 아니라 "**어디까지** 갔는가"다. 문제가 될 수 있는
statement의 집합은 정확히 알고 있다. 그리고 "효과는 남아 있지만 더 이상 어떤 소스의 실행으로도
세지 않는 실행"에는 이미 이름이 있다 — residue다. 부분 실행이 정확히 그것이므로 소스를 통째로
residue에 넣는다. 새 기계가 하나도 필요 없다.

- **과대 기록이 안전한 방향이다.** 실제로는 statement 두 개만 돌았는데 다섯 개를 전부 넣으면
  오염 집합이 실제보다 넓어진다. 넓은 오염 = 더 많은 Run = 낭비일 뿐이고, §1의 비대칭과 같은
  방향이다.
- 위 `x` 예제에 적용하면 residue가 `x = 2`를 담으므로 `x = 1`이 `BindingChanged { x }`로 Run이 된다.
- **순번도 맞는다.** 완주한 부분을 먼저 `realize`하고 끊긴 소스를 `record_partial`하면 부분 실행이
  뒤 순번을 받는다 — §3.4의 전제와 어긋나지 않는다.
- 끊긴 실행은 실현 열에 들어가지 않으므로 **그 자체로는 절대 재사용의 근거가 되지 않는다.**

`poison()`은 그대로 남는다. 둘은 다른 상황을 가리킨다 — `record_partial`은 **어떤 소스가 돌다
끊겼는지는 아는** 경우, `poison`은 세션에 무슨 일이 있었는지 **아무것도** 모르는 경우(사용자가
인터프리터에 직접 붙어 무언가 실행했다)다.

### 3.7 실현 밖 열이 자라지 않게 하기

§3.2의 오염 계산은 실현 밖 실행 **전체**를 훑는다. 그런데 `realize`는 밀려난 실행을 실현 밖으로
옮기기만 하므로, 세션이 오래 살면 판정 결과가 그대로인 채 정렬 비용만 자란다. 실측 — statement
30개짜리 노트북에서 가운데 한 줄을 고치고 실행하는 사이클을 200번 돌리면 실현 밖 실행이 3,000개가
되고 `align`이 **52µs → 2ms**가 된다. 그 200 사이클 내내 `run_steps`는 15로 똑같다.

호출자가 이걸 피할 방법은 세션을 새로 만드는 것뿐인데, 그건 이 라이브러리를 쓰는 이유를 정확히
되돌린다. 그래서 `realize`와 `record_partial`이 끝날 때 **어떤 판정에도 닿을 수 없게 된 실행을
버린다.** 버릴 수 있는 근거는 판정 모델에서 그대로 나오고, 둘 다 오염을 좁히지 않는다.

1. **실현 열 전체보다 앞선 실행은 영원히 무해하다.** 실현 밖 실행이 하는 일은 자기보다 순번이
   앞선 실현 실행의 재사용을 깨는 것뿐이다(§3.4). 실현 열의 모든 실행이 그보다 뒤 순번이면 지금
   깰 것이 없고, 실현 열의 최소 순번은 절대 내려가지 않으므로 — 재사용된 실행은 순번을 그대로
   두고, 다시 실행된 것과 새로 붙는 것은 더 뒤 순번을 받으며, 빠지는 것은 최소를 올릴 뿐이다 —
   앞으로도 없다.
2. **오염 상계가 같은 실행이 뒤에 또 있으면 앞엣것은 덮인다.** 뒤엣것은 순번이 커서 더 많은
   실행을 대상으로 삼고, 별칭 폐포도 그 시점까지 생긴 간선을 전부 보므로 더 넓다. 앞엣것이
   걸리는 자리는 전부 뒤엣것도 걸린다. 그리고 상계는 시간이 지나도 변하지 않는다 — 앞으로
   기록될 요약은 전부 더 뒤 순번이라 이미 지나간 해석을 바꾸지 못한다.

**둘 중 일하는 쪽은 2다.** 1은 직관적이지만 실제 편집 루프에서는 거의 발동하지 않는다 — 이 도구가
잘 동작할수록 노트북 맨 위는 계속 재사용되어 순번을 그대로 유지하고, 그래서 실현 열의 최소 순번이
0에 머문다. 반면 2는 편집 루프의 모양과 정확히 맞는다: 한 줄을 고쳐 가며 반복 실행하면 매번 같은
statement들이 실현 밖으로 밀리고, 그것들의 상계는 같다.

**statement가 달라도 상계가 같으면 덮인다**는 것이 중요하다. 편집 지점 그 줄(`v = base + 1` →
`v = base + 2`)은 canonical이 매번 달라지지만 하는 일은 같은 이름을 다시 묶는 것뿐이라 상계가
같다. canonical 일치로 판정하면 이 줄만 사이클마다 쌓이고, 게다가 **건전하지 않다** — 같은
`f()`라도 사이에 `f`의 재정의가 끼면 두 실행의 상계가 다르다(§3.4의 late binding). 상계 자체로
비교하면 그 함정이 없다.

위 실측에 적용하면 실현 밖 실행이 15개에서 멈추고 `align`이 사이클 수와 무관해진다.

버리는 것은 실행 참조뿐이다. def-use 그래프와 요약 표는 그대로 둔다 — 그쪽은 `produces`를 넓히는
재료이고, 지우면 상계가 좁아져 잘못된 재사용이 된다. `PlanSummary::residue_len`과
`residue_count()`가 누계가 아니라 "지금 판정에 닿을 수 있는 것의 수"인 이유가 이것이다.

---

## 4. 정확성 논증

> **정리.** 실현 열 `H = h₁..h_n`, residue `R`, 입력 `P = p₁..p_m`, `r = prefix_len`,
> `D⁺ = alias_closure(오염 집합)`이라 하자. `d(j) = Reuse ⟺ j < r ∧ produces⁺(p_j) ∩ D⁺ = ∅`,
> 그 외 `Run`이라 하자. §9의 네 가정 하에서, 그리고 **`P`가 self-contained**(`P`가 읽는 모든
> 이름이 builtin이거나 `P` 안에서 먼저 바인딩됨)이면, Run step을 `j` 증가 순으로 실행하면
> `P`가 실현된다.

**Base.** `j=0`에서 두 상태 모두 세션 초기 상태. 자명. ∎

**Step 1 (prefix 실행의 동일성).** `canon(h_i) = canon(p_i)` (i < r)이고 세션은 linear하므로
`h₁..h_r`은 `P`의 처음 r개 statement를 `P`의 순서로, 같은 시작 상태에서 실행한 것이다.
A-Det에 의해 canonical 동일성은 프로그램 동일성이다. 따라서 이 실행들은 **`P`의 처음 r step의
유효한 실현**이다. *값을 재유도하지 않는다 — 실제로 일어난 일이다.* ∎

**Step 2 (오염의 상계).** 실현 밖 실행이 module namespace와 거기서 도달 가능한 객체에 미친
영향은 `D⁺`에 포함된다:

- 이름 재바인딩 — 구문적 `binds`/`deletes`와 세션 정의 함수의 전이 `global_writes`가 덮는다.
  외부 정의 함수는 A-NoForeignGlobalWrite에 의해 우리 global을 재바인딩하지 못한다.
- in-place 변경 — `mutates`(receiver / 인자 / attribute·subscript store / 전이 `mutates_frees`)가
  덮고, 그 밖의 도달 경로는 A-NoAlias가 배제한다.
- 미지 호출 — A-KnownEffects에 의해 결과 바인딩과 언급된 이름 밖으로 새지 않는다.
- 반사적 구문 — 오염 집합을 전체로 만들어 흡수한다. ∎

**Step 3 (Reuse case).** `j < r ∧ produces⁺(p_j) ∩ D⁺ = ∅`이면 Step 1에 의해 `h_j`는 `P`의 `p_j`
실행이고, Step 2에 의해 그 효과를 훼손한 것이 없다. 따라서 지금 `p_j`를 실행하는 것은 `≅`
하에서 no-op이다. 건너뛰어도 상태가 유지된다. ∎

**Step 4 (Run case).** `p_j`를 계획 순서(=`P`의 순서)로 실행한다. `p_j`가 읽는 이름 `x`에 대해:

- `x ∈ D⁺`이고 `P`의 어떤 `p_i` (i < j)가 `x`를 produce하면, `produces⁺(p_i) ∩ D⁺ ≠ ∅`이므로
  `p_i`도 Run이고 `p_j`보다 먼저 실행되어 `x`를 복원한다.
- `x ∉ D⁺`이면 세션의 값이 훼손되지 않았고, Step 1/3에 의해 `P`가 기대하는 값이다.
- `x`가 `P`에서 전혀 바인딩되지 않으면 — self-contained 전제 위반. 이 경우
  `UnresolvedReference` 진단을 발행하고 보증이 **세션 상대적**으로 약해진다. ∎

---

## 5. 실측 반례

전부 실제 Python으로 돌려 확인한 것들이고, `tests/alignment.rs`가 이걸 회귀로 잡는다.

| 반례 | 실측 정답 | 이 알고리즘 |
|---|---|---|
| **late binding** `H=[K=10, def f, K=20, y=f()]`, `P=[K=10, def f, y=f()]` | `y=20` | r=2, residue=`{K=20, y=f()}`, 오염=`{K,y}`. p₁ produces`{K}`가 걸림 → **Run**. p₂ `{f}` → Reuse. p₃ → Run → `f()`가 K=10을 읽어 **20** |
| **인자 mutation** `H=[a=[], def add, add(a), n=len(a)]`, `P=[a=[], def add, n=len(a)]` | `n=0` | `add(a)`의 `mutates_params[0]` → 오염∋`a`. p₁ produces`{a}` → **Run** → `a=[]` 재생성 |
| **데코레이터 레지스트리** `H=[routes=[], def register, @register def hello, n=len(routes)]` | `n=0` | `summary*(register).mutates_frees={routes}` → 오염∋`routes`. p₁ → **Run** |
| **전이 global write** `H=[def g, def h, c=0, h(), z=c]`, `h()` 제거 | `c=0, z=0` | `summary*(h).global_writes={c}` (2단계 전이) → 오염∋`c`. p₃ → **Run** |
| **multiplicity** `add(a)`가 소스에 두 번 | `n=2` | r=3 (위치 고정), residue=`{n=len(a)}`. p₁~p₃ Reuse, p₄·p₅ Run → **2** |
| **컨테이너 별칭** `H=[a=[], keep=a, keep.append(1)]`, `P=[a=[]]` | `a=[]` | `keep=a`가 union(keep,a), `keep.append`가 keep을 변경 → 폐포 → 오염∋`a`. p₁ → **Run** |
| **상속 공유 속성** `H=[class B, class C(B), C.items.append(1), …]` | `n=0` | `class C(B)`가 union(C,B). 오염∋`C` → 폐포∋`B`. p₁ produces`{B}` → **Run** |
| **module 별칭** `H=[import config, from config import LIMIT, config.LIMIT=99]`, 마지막 줄 제거 | 원래 LIMIT | r=2, 오염=`{config}`. p₂ produces`{LIMIT}`은 안 걸림 → **Reuse** |
| **builtins 몽키패치** `H=[import builtins, builtins.len=…, n=len(…)]`, 패치 제거 | `n=3` | 반사 구문이 residue에 → 오염 = 전체 → 전부 **Run** |
| **동일 소스 재정렬** | 전부 재사용 | r=n, residue 비어 있음 → **100% Reuse** |
| **자기 누산** `H=[acc=0, acc=acc+1]`, `P`에 한 줄 더 | `acc=2` | r=2, residue 비어 있음 → 앞 둘 Reuse, 셋째 Run → **2** |
| **비결정성** `x = random()` | — | **건전성 문제가 아니다.** 실현 의미론에서 그 실행이 `P`의 실행이므로 값은 그것이 맞다. 새 값을 원하면 호출자가 `Effect::Nondeterministic`을 보고 후처리한다 |

---

## 6. 상계를 어디까지 잡는가

heap을 모델링하지 않는다. region 분석도, heap-SSA도 없다. statement 하나가 **건드릴 수 있는
이름의 상계**만 잡는다.

### 6.1 compound statement는 원자다

`if` / `for` / `while` / `try` / `with` / `match` / `def` / `class`는 통째로 노드 하나다.
내부를 절대 들여다보지 않는다.

- `binds` = 모든 분기의 may-def **합집합**, `mentions` = 본문에 나타나는 모든 이름.
- **φ-노드가 필요 없다.** 분기 합류가 노드 내부에 숨으므로 SSA 구축이 통째로 사라진다.
- 건전성: canonical이 중첩 본문 전체를 덮으므로 내부의 어떤 변경도 canonical을 바꾼다.
- 부수 효과로 class body 스코프 함정이 닫힌다. 실측:
  `x=10; class C: x=20; ys=[x for _ in range(2)]` → `C.ys == [10, 10]` (comprehension이 class 스코프를 건너뛴다),
  `x=1; class C: y=x; x=2` → `C.y == 1`. 이름 단위로 정밀하게 판정하려 하면 두 경우가 정반대의
  `reads`를 요구한다. 원자화 + 상향 근사면 둘 다 안전한 쪽으로 떨어진다.
- 대가: `for i in range(1000000)`의 절반만 재사용하는 것은 불가능하다. 이건 포기가 아니라
  correctness의 전제다.

### 6.2 mutation과 별칭

```
mutates(s) = { 순수 화이트리스트 밖 호출의 receiver / 인자로 언급한 이름 }
           ∪ { x.attr = v / x[k] = v / del x[k] / x += … 의 root name x }
           ∪ ⋃ { summary*(c).mutates_frees : c ∈ calls(s) }
           ∪ { c의 mutates_params 위치에 넘어간 인자 이름 }
```

**`f(x)`처럼 인자로 넘기는 것만으로 mutation 후보가 된다.** 잔인하지만 참이다 — 실측:
`a=[]; def add(l): l.append(1); add(a)` → `len(a) == 1`. 이전 설계들이 전부 여기서 뚫렸다
(`add(a)`의 writes를 ∅로 봤다).

별칭 간선은 **두 형태에서만** 만든다:

| 형태 | 간선 |
|---|---|
| `b = a` (RHS가 bare `Name`) | `union(b, a)` |
| `class C(Base)` | `union(C, Base)` |

`c = [a]`, `d = f(a)`, `return self._data` 같은 간접 별칭은 잡지 않는다 — A-NoAlias로
명시한다. **이 제한이 결정적이다.** "정의한 이름과 읽은 이름을 전부 union"하는 식으로 넓히면
union-find가 전이적이라 몇 statement 만에 네임스페이스 전체를 한 클래스로 삼켜 재사용률이
0이 된다. bare-name 대입과 상속에만 union을 걸면 퇴화하지 않는다.

`class C(Base)` union의 근거 (실측): `class B: items=[]` / `class C(B): pass` /
`C.items.append(1)` → `len(B.items) == 1`. 상속은 구문상 대응물이 없는 별칭 간선이다.

### 6.3 함수 요약 — 세션에 정의된 것만

호출부가 피호출 함수의 효과를 흡수해야 하는 경우가 셋 있고 전부 실측으로 확인했다 (§5 표의
1~4행). 요약이 필요한 것은 `global_writes`와 `mutates_frees`뿐이다. `reads`는 진단에만 쓰므로
요약하지 않는다.

**세션에 `def`/`class`가 있는 것만** 요약한다. 임포트된 함수와 빌트인은 본문을 볼 수 없으므로
요약이 없고, **A-NoForeignGlobalWrite**가 그 자리를 메운다.

전이 폐포는 방문 집합으로 끝난다 — 이름 집합이 유한하므로 재귀·상호재귀에서도 종료한다.

---

## 7. 정규화 경계 — 무엇을 "같은 statement"로 볼 것인가

**대원칙 (비대칭):** 과다 정규화 = 잘못된 Reuse = 조용히 틀림. 과소 정규화 = 불필요한 Run =
그냥 느림. **애매하면 무조건 "다르다".**

ruff의 `ComparableStmt`가 주는 만큼만 정규화하고 한 걸음도 더 나가지 않는다.

| 정규화한다 | 정규화하지 않는다 |
|---|---|
| 공백 / 개행 / CRLF / 줄 계속 | `1000` vs `1000.0` (타입이 다르다) |
| 주석 | `True` vs `1` |
| `1_000` / `0x3E8` / `1e3` | `-0.0` vs `0.0` (`1/-0.0 == -inf`) |
| `'a'` / `"a"` / `"""a"""` | `f"a"` vs `"a"` (런타임 평가) |
| implicit concat `"a" "b"` vs `"ab"` | `t"a"` vs `f"a"` (PEP 750) |
| `r'\n'` vs `'\\n'`, `b'\x41'` vs `b'A'` | `f"{a}"` vs `f"{a!r}"` |
| tuple 괄호 / 잉여 괄호 / trailing comma | **`f"{a=}"` vs `f"{ a = }"`** (self-documenting `=`는 앞 텍스트를 공백째 출력한다) |
| `if x: pass` 한 줄 ↔ 여러 줄 | alpha-equivalence (`def f(a)` vs `def f(b)` — `__code__.co_varnames`가 다르다) |
| NFKC 식별자 (`𝕏` ↔ `X`) — ruff가 CPython과 동일 | 상수 폴딩 (`2*500` ≠ `1000`) |
| `ExprContext` (Store/Load/Del) | `import os` vs `import os as os` (보수적이라 무해) |

여기에 두 가지를 더 섞는다. **`is_docstring`** — bare string은 부모의 index 0일 때만
`__doc__`이 되는데 `ComparableStmt`는 위치를 모른다. **schema version** — ruff를 올리면 올린다.

**`from __future__`는 canonical에 넣지 않는다.** 그건 statement의 속성이 아니라 세션의
속성이다. prefix 모델에서는 저절로 처리된다 — future 플래그를 바꾸는 statement가 prefix 안에
있으면 양쪽이 동일하고, 밖이면 그 지점부터 전부 Run이다.

**`ComparableStmt`의 유일한 함정:** `ExprContext`를 버린다. statement 단위에서는 `Stmt` variant가
target과 value를 구분하므로 안전하지만, **하위 표현식을 개별 해싱하면 target `a`와 read `a`가
충돌한다.** 그래서 동일성 판정은 statement 단위 canonical로만 하고, def-use는 ctx가 살아 있는
원본 AST에서 별도 스코프 워커로 뽑는다.

해시 충돌이 잘못된 재사용이 되는 경로도 막아 둔다 — digest로 O(1) 걸러내고, 일치하면 encoding
바이트로 확정한다. **이 도구의 오판은 암호학적 가정에 의존하지 않는다.**

---

## 8. 동적 구문 처리 방침

| 구문 | 처리 | 근거 |
|---|---|---|
| **반사 구문 집합** — 이름 `exec` `eval` `globals` `locals` `vars` `setattr` `delattr` `getattr`(비-리터럴) `__import__` `importlib` `builtins`가 statement 어디든 등장, 또는 `sys.modules` / `__dict__` / `__globals__` / `__builtins__` 속성 접근 | `opaque`, `Effect::Opaque`. **residue에 있으면 오염 = 전체 → 전면 Run.** prefix 안에 있으면 무해 (양쪽이 동일하게 실행됐다) | 이것들만이 우리 module globals를 임의로 바꿀 수 있다. 이건 **blacklist**이고 그 사실을 A-NoForeignGlobalWrite와 함께 명시한다 |
| `from m import *` | `opaque` (바인딩 집합을 정적으로 알 수 없다) | 위와 동일 |
| `del x` | `deletes`에 등록 | 정상 def-use. 특수 케이스 코드 없음 |
| **compound** (`if`/`for`/`while`/`try`/`with`/`match`) | 원자 노드 1개 | §6.1 |
| `def` / `class` | 원자 노드 + 요약 | §6.3 |
| 조건부 import (`try: import cupy as xp / except: import numpy as xp`) | compound 원자성이 그대로 처리 — `binds = {xp}` | 분기를 쪼개지 않으므로 보수적이고 정확 |
| 데코레이터 `@app.route(...)` | 데코레이터 식의 이름을 `mentions`+`calls`에 등록. 세션 정의면 전이 흡수 | 실측 확인 (§5 표 3행) |
| 클래스 상속 `class C(Base)` | `mentions ∋ Base`, **별칭 간선 `(C, Base)`** | 실측: 상속된 mutable 속성은 공유 객체다 (§6.2) |
| 함수 내 `global g` | 요약의 `global_writes`, 호출부가 전이 폐포로 흡수 | 실측: 2단계 전이도 잡는다 |
| `b = a` 후 `b.mutate()` | 별칭 클래스, 오염 집합의 폐포에만 사용 | bare-Name 대입에만 union → 퇴화하지 않음 |
| `f(x)` (미지 호출) | `mutates ∋ x`. global 재바인딩은 없다고 본다 | A-NoForeignGlobalWrite |
| `x.m(...)` (순수 화이트리스트 밖) | `mutates ∋ x` + 인자 전부 | 상계. false run 방향 |
| IPython magic (`%time`, `!ls`, `?obj`) | `ParseError` | 순수 Python만 파싱한다. 셸이 cell에서 magic을 금지한다는 전제 |
| `assert` | 정상 statement. `-O`로 무력화되는 것은 모델 밖 | §10 |
| PEP 263 non-UTF8 cookie | `ParseErrorKind::UnsupportedEncoding` — 거부 | `Range`가 어느 바이트열 기준인지 흐려지는 것보다 명시적 거부가 낫다 |

---

## 9. 서 있는 가정 — 정확히 4개

정적 분석만으로 Python을 건전하게 다루는 것은 불가능하다. 어떤 설계든 가정 위에 선다.
유일하게 정직한 선택은 **가정의 개수를 세고, 각각을 사용자가 판단할 수 있는 문장으로 쓰는
것**이다.

| | 가정 | 깨지면 | 이 설계의 방어 |
|---|---|---|---|
| **A-Det** | canonical하게 같은 statement를 같은 순서로 같은 시작 상태에서 실행하면 같은 상태가 된다 | prefix 동일성이 무효 | 실현 의미론에서는 **비결정성이 건전성 문제가 아니다**. 외부 세계가 정렬 사이에 바뀐 경우만 실질 위험 → `Effect::ExternalRead`로 노출하고 판단은 호출자에게 넘긴다 |
| **A-NoAlias** | 객체는 statement가 구문적으로 언급한 이름, bare-name 대입 별칭, 클래스 상속 관계를 통해서만 변경된다 | 잘못된 reuse | 상계를 넓게 잡고, 잡을 수 있는 두 별칭 형태는 실제로 잡는다. 나머지는 문서화된 한계 |
| **A-NoForeignGlobalWrite** | 세션 밖에 정의된 함수는 이 module의 global을 재바인딩하지 않는다 | 잘못된 reuse | Python 의미론상 참이다 (`global x`는 그 함수 module의 x를 쓴다). 예외는 반사 구문뿐이고 §8이 오염 = 전체로 흡수한다 |
| **A-KnownEffects** | 순수 화이트리스트 밖 호출의 효과는 (결과 바인딩 ∪ 언급된 이름의 in-place 변경 ∪ 세션 정의 callee의 전이 요약) 안에 있다 | 잘못된 reuse | 언급된 이름 전부를 mutation 후보로 잡는 극단적 상계. false run 방향으로만 틀린다 |

§5 표의 12개 반례 중 10개는 알고리즘이 정확히 처리하고, 남은 2개(간접 별칭, 고차 함수)는
A-NoAlias에 정확히 대응한다. **가정이 있다는 것이 문제가 아니라, 가정 목록이 실제 구멍의
일부만 덮는 것이 문제다.**

---

## 10. v0.1이 명시적으로 포기한 것

1. **간접 별칭** — `c = [a]`, `d = f(a)`, `return self._data`, `dict.setdefault`, `a, b = pair`. A-NoAlias.
2. **고차 함수 / 클로저 / 동적 디스패치** — `h = f; h()`, `fs[0]()`, `getattr(m,'f')()`. 호출 대상이 정적으로 안 잡히면 요약을 흡수하지 못한다. `mutates` 상계만 적용된다.
3. **인터프리터 플래그** — `-O`(assert 무력화), `-OO`(docstring 제거), `PYTHONHASHSEED`.
4. **C 확장 내부 상태**, 프로세스 밖에서 바뀌는 외부 파일.
5. **compound 내부 부분 재사용** — §6.1. 포기가 아니라 correctness의 전제다.
6. **디스크 영속화** — 인메모리 전용. canonical encoding에 schema version을 섞어 두어 나중에 열 때 옛 세션이 자동 무효화되게만 해 뒀다.
7. **fork / rewind** — linear 모델을 그대로 따른다.

### 부분 실행

**어디까지 갔는지는 표현하지 않는다.** `for p in paths: rows.append(load(p))`가 4번째에서 죽으면
인터프리터에는 statement 하나의 절반이 남는데, 그 절반을 세션에 적을 방법이 없다. 대신 끊긴
소스를 통째로 residue에 넣어 상계로 흡수한다 (§3.6). 아는 척하지 않는 것이 여기서의 포기이고,
대가는 필요 이상의 Run뿐이다.

---

## 11. ruff를 정확히 핀하는 이유

`ruff_python_parser` / `ruff_python_ast` / `ruff_text_size`는 `=`로 **정확히 핀**한다.

- 버전이 섞이면 `ruff_python_ast`가 의존성 트리에 두 벌 들어가 타입이 안 맞는다. 0.0.x는
  cargo가 전부 semver-incompatible로 취급하므로 업그레이드가 항상 수동 = 사고가 없다.
- ruff 0.0.x는 공식적으로 "no stability guarantee"다. 보험은 **ruff 타입이 public API에 한 글자도
  없다**는 것 하나다 — ruff의 breaking change가 이 crate의 semver를 인질로 잡지 못하고 patch
  릴리스로 흡수된다.
- 진짜 fallback은 다른 파서가 아니라 **같은 ruff crate를 git dependency로 받는 것**이다 (코드 변경 0).
- 그 대가로 MSRV가 최신 stable 근처를 따라간다. `rust-toolchain.toml`이 정본이다.
