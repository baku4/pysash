use super::Effect;

/// statement 하나에서 정적으로 뽑아낸 사실. 오염 집합 계산의 재료다.
///
/// 전부 상향 근사다 — 실제보다 넓게 잡을 수는 있어도 좁게 잡지는 않는다.
/// 좁게 잡으면 잘못된 재사용(조용히 틀린 결과)이 되기 때문이다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatementFacts {
    /// module namespace에 바인딩할 수 있는 이름 (may-def 합집합).
    pub binds: Vec<Box<str>>,
    /// 읽는 module-level free name.
    pub reads: Vec<Box<str>>,
    /// 언급하는 모든 이름. mutation 상계의 기반이다.
    pub mentions: Vec<Box<str>>,
    /// `b = a` (RHS가 bare Name) / `class C(Base)` 로 생기는 별칭 간선.
    pub alias_edges: Vec<(Box<str>, Box<str>)>,
    /// 직접 호출하는, 정적으로 이름이 잡히는 대상.
    pub calls: Vec<Box<str>>,
    /// `def`/`class`일 때의 본문 요약. 호출부가 이걸 흡수한다.
    pub summary: Option<CalleeSummary>,
    /// `del x`로 지우는 이름.
    pub deletes: Vec<Box<str>>,
    /// 이 statement가 in-place로 바꿀 수 있는 이름의 상계 — attr/subscript 대입의
    /// root, augmented assign의 root, 순수 화이트리스트 밖 호출의 receiver와 인자.
    pub mutates: Vec<Box<str>>,
    pub effect: Effect,
    /// 반사적 구문 — prefix 밖에 있으면 오염 집합이 전체가 된다.
    pub opaque: bool,
}

/// `def`/`class` 본문의 요약. 호출하면 무슨 일이 일어나는지의 상계.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CalleeSummary {
    /// 본문의 `global x; x = ...` — 호출하면 module global x가 바인딩된다.
    pub global_writes: Vec<Box<str>>,
    /// 본문이 in-place로 바꾸는 free name.
    pub mutates_frees: Vec<Box<str>>,
    /// 본문이 in-place로 바꾸는 파라미터 위치.
    pub mutates_params: Vec<usize>,
    /// 본문이 호출하는 대상. 전이 폐포 계산용.
    pub callees: Vec<Box<str>>,
    /// 본문에 반사적 구문이 있다 — 이 함수 호출은 무엇이든 할 수 있다.
    pub opaque: bool,
}

impl CalleeSummary {
    /// 다른 callable의 요약을 이 요약에 합친다 — 이 callable을 호출하면 저쪽이
    /// 하는 일도 일어날 수 있다는 뜻이 된다.
    pub fn absorb(&mut self, other: &CalleeSummary) {
        for name in &other.global_writes {
            if !self.global_writes.contains(name) {
                self.global_writes.push(name.clone());
            }
        }
        for name in &other.mutates_frees {
            if !self.mutates_frees.contains(name) {
                self.mutates_frees.push(name.clone());
            }
        }
        for position in &other.mutates_params {
            if !self.mutates_params.contains(position) {
                self.mutates_params.push(*position);
            }
        }
        for name in &other.callees {
            if !self.callees.contains(name) {
                self.callees.push(name.clone());
            }
        }
        self.opaque |= other.opaque;
    }
}

impl Default for StatementFacts {
    /// 아무것도 알아내지 못한 statement. 모른다는 것은 무엇이든 할 수 있다는
    /// 뜻이므로 반사적으로 취급한다 — 틀려도 Run이 늘어날 뿐이다.
    fn default() -> Self {
        Self {
            binds: Vec::new(),
            reads: Vec::new(),
            mentions: Vec::new(),
            alias_edges: Vec::new(),
            calls: Vec::new(),
            summary: None,
            deletes: Vec::new(),
            mutates: Vec::new(),
            effect: Effect::Opaque,
            opaque: true,
        }
    }
}
