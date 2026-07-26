/// 소스를 어느 문법으로 읽는가.
///
/// `Ipython` 모드에서만 `%timeit` / `!ls` / `?obj` 가 statement로 파싱된다.
/// 같은 바이트열이라도 모드가 다르면 다른 statement이므로, 모드는
/// `CanonicalStatement`의 동일성 판정에 섞인다.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum SourceMode {
    #[default]
    Python,
    Ipython,
}
