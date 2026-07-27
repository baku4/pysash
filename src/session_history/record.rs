use crate::python_source::PythonSource;
use super::SessionHistory;

impl SessionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// 성공한 실행 하나를 기록 끝에 잇는다.
    ///
    /// 계획을 수행한 뒤라면 **실제로 다시 실행한 부분만** 넣는다. 그러면 세션의
    /// 꼬리가 정확히 그 소스가 되어, 같은 소스를 다시 정렬했을 때 할 일이 없다.
    /// 실행하지 않은 것을 넣으면 기록이 거짓이 되고 판정도 따라서 틀린다.
    pub fn push(&mut self, code: &PythonSource) {
        self.sources.push(code.clone());
    }

    /// 입력된 소스들. 순서 그대로다.
    pub fn sources(&self) -> &[PythonSource] {
        &self.sources
    }
}
