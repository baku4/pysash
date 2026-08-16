# Open issues

This file records issues intentionally deferred outside the alignment model. Design decisions belong
in [design.md](design.md).

## 1. Exact Ruff pins keep the MSRV near current stable

**Status:** Open; one reason crates.io publication remains deferred.

`Cargo.toml` pins `ruff_python_parser`, `ruff_python_ast`, and `ruff_text_size` to `=0.0.6` and follows
their recent MSRV (currently Rust 1.95 with toolchain 1.97.1). This is frictionless for the current
single controlled consumer but creates two costs for general users:

- An exact pin cannot unify with another Ruff version already present in a consumer's dependency
  tree.
- Ruff 0.0.x makes no stability guarantee and has raised its MSRV frequently, excluding consumers
  tied to older distribution or enterprise toolchains.

The pin is intentional; see [design section 11](design.md#11-why-the-ruff-crates-are-pinned-exactly).
The unstable upstream crate, rather than the pin itself, is the unresolved constraint.

Revisit this issue when Ruff offers a semver-compatible release range or its MSRV cadence settles.
If an external consumer arrives first, the fallback is the same Ruff version as a Git dependency,
which requires no PySASH code change.

## 2. Source retention through positional references

**Status:** Resolved; formerly another publication blocker.

The old history stored sources in an array and referred to them by index. Residue compaction could
drop an execution but not its source because removing an array element would shift every later
reference. Repeated edits therefore retained one full source version per cycle.

`ExecRef` now owns a shared `PythonSource` clone. Dropping the last execution reference releases the
source automatically. The former `SessionHistory::sources()` API was removed, and
`SessionDiagnostic::OpaqueResidue` carries statement text instead of a source index and range.

`forgotten_executions_release_their_sources` verifies that 50 edit cycles retain a constant number of
source allocations: 3 in the measured case instead of 51 with the old representation.
