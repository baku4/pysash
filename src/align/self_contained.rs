use std::collections::HashSet;
use crate::plan::StatementDiagnostic;
use crate::statement::Statement;

/// Names provided by a standard Python interpreter.
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

/// Returns unresolved reads for each statement at the corresponding output position.
pub fn unresolved_reads(statements: &[Statement]) -> Vec<Vec<StatementDiagnostic>> {
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
                .map(|name| StatementDiagnostic::UnresolvedReference { name: name.clone() })
                .collect()
        })
        .collect()
}
