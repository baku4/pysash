use crate::statement::Statement;

/// 세션의 **끝**과 소스의 **앞**이 몇 개나 겹치는가.
///
/// 세션의 마지막 `m`개가 소스의 첫 `m`개와 같으면, 그 `m`개는 방금 이 소스를
/// 그만큼 실행한 것이고 **그 뒤에 아무 일도 없었다.** 꼬리에 맞췄으니 뒤에 아무것도
/// 없다는 것은 정의상 참이다 — 여기에는 가정이 하나도 없다.
///
/// 세션의 **앞**에서부터 맞추면 이 성질이 없다. 세션이 한 발짝만 더 나가도 그
/// 뒤에 무슨 일이 있었는지 알 수 없어 전부 버려야 한다.
///
/// 위치로 고정해 비교한다. "어딘가 같은 statement가 있는가"를 묻지 않는다 —
/// 세션은 linear하므로 순서가 곧 의미다.
pub fn overlap_len(session: &[&Statement], code: &[Statement]) -> usize {
    let limit = session.len().min(code.len());
    (0..=limit)
        .rev()
        .find(|&reused| {
            session[session.len() - reused..]
                .iter()
                .zip(code)
                .all(|(a, b)| a.canonical == b.canonical)
        })
        .unwrap_or(0)
}
