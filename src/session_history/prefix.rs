use crate::statement::Statement;

/// 두 statement 열이 앞에서부터 몇 개나 canonical하게 같은가.
///
/// **위치로 고정한다.** 전역 조회로 "어딘가 같은 statement가 있는가"를 묻지 않는다.
/// 같은 statement가 두 번 나오면 그건 두 개의 다른 실행이고, 순서가 바뀌면
/// 공통 prefix는 0이다.
pub fn prefix_len(session: &[Statement], code: &[Statement]) -> usize {
    session
        .iter()
        .zip(code)
        .take_while(|(a, b)| a.canonical == b.canonical)
        .count()
}
