/// 이 statement가 무엇을 하는가의 분류. 판정이 아니라 사실이다.
///
/// 재사용 판정의 게이트가 아니다 — 판정은 오염 집합이 한다. 이 분류는 호출자가
/// plan을 후처리할 때 쓴다. 예를 들어 외부 파일이 바뀌었을 수 있으니
/// [`ExternalRead`](Effect::ExternalRead)는 재사용하지 않겠다는 정책은 호출자의
/// 몫이다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// 이름 바인딩 외에 관측 가능한 효과 없음.
    Pure,
    /// 모듈 임포트. `sys.modules` 캐시 덕에 재실행이 멱등이다.
    Import,
    /// print / display / 로깅 같은 출력.
    Output,
    /// 파일·네트워크·`input()` — 외부 세계를 읽는다.
    ExternalRead,
    /// 파일 쓰기·네트워크 전송·subprocess — 외부 세계를 바꾼다.
    ExternalWrite,
    /// random / time / uuid 같은 비결정적 값 생성.
    Nondeterministic,
    /// 반사적 구문 — 무엇이든 할 수 있다.
    Opaque,
}
