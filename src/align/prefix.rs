use crate::statement::Statement;

/// 실현 열의 앞과 소스의 앞이 위치 고정으로 몇 개나 같은가.
///
/// 위치로 고정해 비교한다. "어딘가 같은 statement가 있는가"를 묻지 않는다 —
/// 세션은 linear하므로 순서가 곧 의미다. 같은 statement가 여러 번 나와도 각각
/// 다른 위치의 다른 실행이다.
pub fn prefix_len(realized: &[&Statement], code: &[Statement]) -> usize {
    realized
        .iter()
        .zip(code)
        .take_while(|(executed, incoming)| executed.canonical == incoming.canonical)
        .count()
}
