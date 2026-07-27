use crate::python_source::PythonSource;
use super::SessionHistory;
use super::prefix::prefix_len;

impl SessionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// 성공한 실행을 기록 끝에 잇는다.
    pub fn push(&mut self, code: &PythonSource) {
        self.realized.extend_from_slice(code.statements());
    }

    /// `code`의 계획을 실행 완료했음을 기록한다.
    ///
    /// plan을 인자로 받지 않고 안에서 prefix를 다시 계산한다 — 위조된 plan이
    /// 세션을 오염시킬 경로 자체가 없다.
    ///
    /// 공통 prefix를 넘어 세션이 실행했던 것들은 `residue`로 밀려난다. 그것들이
    /// 남긴 효과는 되돌릴 수 없으므로 계속 기억해야 한다.
    pub fn realize(&mut self, code: &PythonSource) {
        let common = prefix_len(&self.realized, code.statements());
        let displaced = self.realized.split_off(common);
        self.residue.extend(displaced);
        self.realized = code.statements().to_vec();
    }
}
