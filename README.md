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

## MSRV

The MSRV follows near-current stable Rust because the Ruff parser crates are pinned exactly.
[`rust-toolchain.toml`](rust-toolchain.toml) is authoritative.
