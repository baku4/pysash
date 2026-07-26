use super::Effect;

/// statement 하나에서 뽑아낸 정적 사실. 의존성 그래프와 오염 집합의 재료다.
///
/// 이름 집합은 전부 **상계**다 — 실제로 건드리는 것보다 넓게 잡는다. 넓게 잡으면
/// 불필요한 Run이 늘 뿐이지만, 좁게 잡으면 조용히 틀린 재사용이 된다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StatementFacts {
    /// module namespace에 바인딩할 수 있는 이름. 모든 분기의 may-def 합집합이다.
    pub binds: Vec<Box<str>>,
    /// 읽는 module-level free name.
    pub reads: Vec<Box<str>>,
    /// **언급하는** 모든 이름. mutation 상계의 기반이다 — 인자로 넘기기만 해도
    /// 그 객체는 바뀔 수 있다.
    pub mentions: Vec<Box<str>>,
    /// `b = a` (RHS가 bare name) 와 `class C(B)` 로 생기는 별칭 간선 `(왼쪽, 오른쪽)`.
    /// 이 두 형태에만 거는 이유는 전이 폐포가 네임스페이스 전체를 삼키지 않게 하기 위해서다.
    pub alias_edges: Vec<(Box<str>, Box<str>)>,
    /// 직접 호출하는, 정적으로 이름이 잡히는 대상.
    pub calls: Vec<Box<str>>,
    /// `del x`.
    pub deletes: Vec<Box<str>>,
    /// `def` / `class` 일 때의 요약. 호출부가 이것을 전이적으로 흡수한다.
    pub summary: Option<CalleeSummary>,
    pub effect: Effect,
    /// 반사적 구문이다. 잉여 실행에 있으면 오염 집합이 ⊤가 된다.
    pub opaque: bool,
    /// bare string literal이 부모의 첫 statement인가. 그때만 `__doc__`이 된다.
    pub is_docstring: bool,
    /// `from __future__ import ...` 가 켜는 플래그 비트.
    pub future_flags: u16,
}

/// 세션 안에서 `def` / `class` 로 정의된 대상의 요약.
///
/// 본문을 볼 수 있는 것만 요약한다. 임포트된 함수는 요약이 없고, 대신 "세션 밖에
/// 정의된 함수는 이 module의 global을 재바인딩하지 않는다"는 가정이 그 자리를
/// 메운다 — `global x`는 그 함수가 속한 module의 `x`를 쓰기 때문이다.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CalleeSummary {
    /// 본문의 `global x; x = ...`. 이 대상을 호출하면 module global이 바인딩된다.
    pub global_writes: Vec<Box<str>>,
    /// 본문이 in-place로 바꾸는 free name. 데코레이터의 `registry.append(f)` 같은 것.
    pub mutates_frees: Vec<Box<str>>,
    /// 본문이 in-place로 바꾸는 파라미터 위치.
    pub mutates_params: Vec<usize>,
    /// 이 대상이 호출하는 것들. 전이 폐포 계산에 쓴다.
    pub callees: Vec<Box<str>>,
    /// 본문에 반사적 구문이 있다. 이 대상의 호출은 opaque다.
    pub opaque: bool,
}
