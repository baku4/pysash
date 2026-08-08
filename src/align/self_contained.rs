use std::collections::HashSet;
use crate::diagnostic::Diagnostic;
use crate::statement::Statement;

/// Python 인터프리터가 기본으로 제공하는 이름들. 이 이름을 읽는 것은 소스 밖
/// 의존이 아니다.
const BUILTINS: &[&str] = &[
    "ArithmeticError", "AssertionError", "AttributeError", "BaseException",
    "BaseExceptionGroup", "BlockingIOError", "BrokenPipeError", "BufferError", "BytesWarning",
    "ChildProcessError", "ConnectionAbortedError", "ConnectionError", "ConnectionRefusedError",
    "ConnectionResetError", "DeprecationWarning", "EOFError", "Ellipsis", "EnvironmentError",
    "Exception", "ExceptionGroup", "FileExistsError", "FileNotFoundError", "FloatingPointError",
    "FutureWarning", "GeneratorExit", "IOError", "ImportError", "ImportWarning",
    "IndentationError", "IndexError", "InterruptedError", "IsADirectoryError", "KeyError",
    "KeyboardInterrupt", "LookupError", "MemoryError", "ModuleNotFoundError", "NameError",
    "NotADirectoryError", "NotImplemented", "NotImplementedError", "OSError", "OverflowError",
    "PendingDeprecationWarning", "PermissionError", "ProcessLookupError", "RecursionError",
    "ReferenceError", "ResourceWarning", "RuntimeError", "RuntimeWarning", "StopAsyncIteration",
    "StopIteration", "SyntaxError", "SyntaxWarning", "SystemError", "SystemExit", "TabError",
    "TimeoutError", "TypeError", "UnboundLocalError", "UnicodeDecodeError", "UnicodeEncodeError",
    "UnicodeError", "UnicodeTranslateError", "UnicodeWarning", "UserWarning", "ValueError",
    "Warning", "ZeroDivisionError", "__builtins__", "__debug__", "__doc__", "__file__",
    "__import__", "__name__", "abs", "aiter", "all", "anext", "any", "ascii", "bin", "bool",
    "breakpoint", "bytearray", "bytes", "callable", "chr", "classmethod", "compile", "complex",
    "delattr", "dict", "dir", "divmod", "enumerate", "eval", "exec", "exit", "filter", "float",
    "format", "frozenset", "getattr", "globals", "hasattr", "hash", "help", "hex", "id", "input",
    "int", "isinstance", "issubclass", "iter", "len", "list", "locals", "map", "max",
    "memoryview", "min", "next", "object", "oct", "open", "ord", "pow", "print", "property",
    "quit", "range", "repr", "reversed", "round", "set", "setattr", "slice", "sorted",
    "staticmethod", "str", "sum", "super", "tuple", "type", "vars", "zip",
];

/// 각 statement가 읽지만, 이 소스 어디에서도 바인딩되지 않고 builtin도 아닌 이름.
///
/// 세션에 있어야만 실행되는 조각이라는 신호다 — fresh run에서는 재현되지 않는다.
/// `range`는 이름의 위치가 아니라 그 statement의 위치다.
pub fn unresolved_reads(statements: &[Statement]) -> Vec<Vec<Diagnostic>> {
    let bound: HashSet<&str> = statements
        .iter()
        .flat_map(|statement| statement.facts.binds.iter().map(|name| &**name))
        .collect();
    statements
        .iter()
        .map(|statement| {
            statement
                .facts
                .reads
                .iter()
                .filter(|name| !bound.contains(&***name) && !BUILTINS.contains(&&***name))
                .map(|name| Diagnostic::UnresolvedReference {
                    name: name.clone(),
                    range: statement.range,
                })
                .collect()
        })
        .collect()
}
