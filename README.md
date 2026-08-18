# PySASH

PySASH statically aligns Python source with a linear session history and decides which statement executions can be reused.

```text
SessionHistory + Python source
            |
            v
      Reuse | Run
```

Run the canonical edit-loop tutorial with `cargo run --example align`. PySASH never executes
Python; the caller runs each `Run` step and records successful completion. See
[docs/design.md](docs/design.md) for the reasoning and correctness argument.

## Assumptions

| Name | Required contract | Example violation |
|---|---|---|
| **A-Det** | Canonically equal statements executed in the same order from the same state produce the same state. | An external file changes between alignments. |
| **A-NoAlias** | Objects change only through syntactically mentioned names, bare-name aliases, or class inheritance. | `c = [a]; c[0].append(1)` indirectly mutates `a`. |
| **A-NoForeignGlobalWrite** | Functions defined outside the session do not rebind this module's globals. | External reflective code writes through this module's `globals()`. |
| **A-KnownEffects** | Unknown calls stay within result bindings, mentioned-object mutation, and transitive summaries of session-defined callees. | `h = f; h()` hides a callee's global write. |

Violating an assumption can permit incorrect reuse. Conservative uncertainty instead produces
extra `Run` steps.

## Out of scope

- IPython magic such as `%time` and `!ls`; input must be Python.
- Partial reuse inside a compound statement; a loop or function definition is one statement.
- Persistent histories, fork, and rewind.

## Python syntax

PySASH accepts what CPython's `ast.parse` accepts, not what `compile` accepts. Grammatical
source that no interpreter would run, such as `return` outside a function or a duplicate
parameter, still becomes statements; the session fails to execute them, records nothing, and
alignment reaches `Run`.

The grammar spans every construct CPython added through 3.14 plus the 3.15 syntax already in
preview, and does not gate on a target version: source a session's interpreter would reject
still parses. The two directions are not symmetric, because rejecting syntax the session can
execute makes the tool unusable, while accepting syntax it cannot execute only costs an extra
`Run`.

Coverage comes entirely from the pinned Ruff parser.
[`tests/python_syntax.rs`](tests/python_syntax.rs) records it along three axes — the release
that changed each construct, every reachable AST node kind, and grammatical source no
interpreter will run — and fails when the pin changes any of them.

## MSRV

The MSRV follows near-current stable Rust because the Ruff parser crates are pinned exactly.
[`rust-toolchain.toml`](rust-toolchain.toml) is authoritative.
