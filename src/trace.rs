use super::source::PythonSource;
use super::statement::Statement;

/// 실행 하나 — 어느 소스의 몇 번째 statement였고, 세션 전체에서 몇 번째로
/// 실행되었는지.
///
/// 소스를 **직접 소유한다** (`PythonSource`는 전부 `Arc` 백업이라 clone이 O(1)).
/// 세션이 소스 목록을 따로 들고 위치로 가리키면, 죽은 소스를 버릴 때 뒤 위치가
/// 전부 밀려 참조가 다른 소스를 가리킨다. 실행이 소스를 들면 실행을 버리는 순간
/// 그 소스의 마지막 소유가 풀려 저절로 해제된다 — 별도의 정리가 필요 없다.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecRef {
    /// 이 실행이 속한 소스.
    pub source: PythonSource,
    /// 그 소스 안에서 몇 번째 statement인가.
    pub index: usize,
    /// 세션 전체에서 몇 번째 실행인가. 실현 열이 재배열되어도 실제로 일어난
    /// 순서는 이 값이 기억한다 — 오염은 시간을 거슬러 일어나지 않는다.
    pub seq: usize,
}

impl ExecRef {
    /// 이 실행이 실행한 statement.
    pub fn statement(&self) -> &Statement {
        &self.source.statements()[self.index]
    }

    /// 이 실행이 실행한 statement의 원문.
    pub fn text(&self) -> &[u8] {
        self.source.slice(self.statement().range)
    }
}
