use super::source::PythonSource;
use super::statement::Statement;

/// 실행 하나가 어느 소스의 몇 번째 statement였고, 세션 전체에서 몇 번째로
/// 실행되었는지.
///
/// 세션은 소스를 통째로 보관하므로, 실행 열은 이 참조의 나열로 충분하다.
/// `SessionHistory → PythonSource → statement` 트리가 이걸로 보존된다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExecRef {
    /// 세션에 입력된 몇 번째 소스인가.
    pub source: usize,
    /// 그 소스 안에서 몇 번째 statement인가.
    pub index: usize,
    /// 세션 전체에서 몇 번째 실행인가. 실현 열이 재배열되어도 실제로 일어난
    /// 순서는 이 값이 기억한다 — 오염은 시간을 거슬러 일어나지 않는다.
    pub seq: usize,
}

impl ExecRef {
    /// 이 실행이 실행한 statement.
    pub fn statement<'a>(&self, sources: &'a [PythonSource]) -> &'a Statement {
        &sources[self.source].statements()[self.index]
    }
}
