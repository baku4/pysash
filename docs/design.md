# Alignment design

This document explains the decisions behind PySASH. Rustdoc defines individual API contracts;
this document owns the algorithm, bounds, correctness argument, counterexamples, and assumptions.

## 1. Safety asymmetry

Incorrect `Reuse` can silently produce a wrong result. An unnecessary `Run` only costs time.
Every reuse therefore needs positive evidence, and uncertainty becomes `Run`.

The control flow encodes this rule: `Action::Reuse` is constructed in exactly one branch, for an
undisturbed execution in the common prefix. All other branches return `Run`. An all-`Run` plan is
structurally valid, although recreating the intended starting state for session-relative source may
require a fresh interpreter.

## 2. What reuse means

Reuse means that an execution already was the execution of this statement at this position. It does
not mean that recomputing the statement would probably return the same value.

### 2.1 Realization

Let `P = p1..pm` be the input source, `d` its plan, and `phi` map every reused step to a session
execution. The plan realizes `P` when this execution sequence is a valid execution of `P`:

```text
for j = 1..m:
    reuse exec(h_phi(j)) when d(j) = Reuse
    execute p_j          when d(j) = Run
```

Each reused execution must have occurred in an observationally equivalent state, and no later
execution may have damaged its effect.

A correct plan produces a state that agrees with this realization on every name bound by `P`:

```text
for every n in binds(P):  sigma_plan(n) ~= sigma_real(n)
dom(sigma_plan)           >= dom(sigma_real)
```

The domain is a super-set because bindings left by out-of-sequence session work may remain present.

The rejected alternative was append semantics: execute all of `P` on top of the current session and
call that the target. Under append semantics, aligning identical source containing
`acc = acc + 1` must run it again, so the primary reuse case disappears. Realization semantics makes
identical-source alignment fully reusable and treats `x = random()` as an already valid execution.
Callers that want a fresh random value can downgrade its `Effect::Nondeterministic` step.

### 2.2 Why the witness is a prefix

Earlier designs tried to match fingerprints derived from canonical code, epochs, and dependency
versions. That approach must reconstruct value equivalence from complete statement inputs and
outputs, which Python does not expose statically: even the output set of `add(a)` is unknown without
modeling arbitrary mutation.

If the first `r` realized statements are canonically equal to the first `r` input statements, they
are the same program executed in the same order from the same initial state. Their values do not
need reconstruction; they are the actual executions. The only remaining question is whether later
session work disturbed their effects.

Prefix matching is positional. `H = [a; b]` and `P = [b; a]` have an empty prefix, and repeated
statements at different positions have different witnesses. This is the crate's intentional linear
session model.

## 3. Algorithm

Prefix matching is a single positional pass. The decision step then checks each of the `r`
prefix candidates against every retained residue entry, computing a time-bounded alias closure
per pair, so alignment costs `O(r * |R| * alias)` rather than `O(n + m)`. Section 3.6 keeps
`|R|` bounded so this stays proportional to the current source rather than the session's age.
There is no recursive fixpoint.

### 3.1 Match the positional prefix

```text
r = 0
while r < realized.len and r < statements.len
      and realized[r].canonical == statements[r].canonical:
    r += 1
```

The comparison uses the full canonical encoding after a digest precheck, so hash collisions cannot
authorize reuse.

### 3.2 Build residue disturbance bounds

Residue consists of the realized suffix `realized[r..]` plus retained executions displaced by older
edits or recorded as partial. For each residue execution `e`:

```text
rebound(e) = binds(e) U deletes(e)
             U union(summary*(c).global_writes for c in calls(e))

mutated(e) = mutates(e)
             U union(summary*(c).mutates_frees for c in calls(e))

opaque(e)  = facts(e).opaque
             OR any(summary*(c).opaque for c in calls(e))
```

`summary*` is the transitive summary of the definition live when `e` executed. Poisoned state or an
opaque later execution removes all positive reuse evidence.

For a candidate statement `s`:

```text
produces(s) = binds(s) U deletes(s) U mutates(s)
              U union(summary*(c).global_writes for c in calls(s))
              U union(summary*(c).mutates_frees for c in calls(s))
```

Rebinding compares exact names. In-place mutation also follows alias edges that existed at the
mutation's execution sequence; rebinding a name does not mutate the object referenced by an alias.

### 3.3 Decide every statement

```text
if poisoned:
    (Run, NoMatchingExecution)
else if j < r:
    no later disturbance       -> (Reuse, ReusableExecution)
    later rebinding of n       -> (Run, BindingChanged { n })
    later mutation through n   -> (Run, DependencyChanged { n })
    unknown later effect       -> (Run, NoMatchingExecution)
else if j == r and r < realized.len:
    (Run, StatementChanged)
else if matching code occurred at another position:
    (Run, DependencyChanged { first_read })
else:
    (Run, NoMatchingExecution)
```

The five `DecisionReason` variants cover this decision tree. A matching statement at another
position is not a witness because its execution context differs.

### 3.4 Time is part of disturbance

Ignoring time prevents the edit loop from converging: after `realize` displaces an old execution,
that old execution would appear to disturb the newer execution that replaced it. Every execution
therefore receives a monotonic global sequence number, and residue can disturb only executions with
smaller sequence numbers.

Two other structures use the same boundary:

- Callable summaries are stored as `(definition sequence, summary)` versions. A call resolves to
  the latest definition preceding its own execution, matching Python late binding.
- Alias closure uses only edges created before the disturbance. A later `p = a` cannot propagate an
  earlier mutation backward in time.

Generated-session property tests found both counterexamples. A union-find was rejected because it
cannot answer closure at a historical sequence.

### 3.5 Updating the session

- `push` appends source known to have executed successfully, matching a REPL workflow.
- `realize` recomputes the plan, replaces rerun prefix entries with new executions, appends the
  remaining statements, and moves displaced executions to residue.
- `record_partial` adds every statement of interrupted source to residue.
- `poison` marks changes whose source is completely unknown; recovery requires a new interpreter
  and `SessionHistory`.

`realize` accepts source rather than a caller-supplied plan, so a fabricated plan cannot enter the
history. Sequence allocation, def-use recording, and summary recording occur in one function. A
rerun definition must receive a new version; otherwise an intervening old redefinition could remain
the apparent definition for later calls and narrow their effect bound.

Partial execution deliberately over-approximates. IPython may report a failed cell without a
trustworthy statement index, but the complete cell source is known. Recording all its statements can
only enlarge disturbance and cause extra execution. Any completed prefix must be recorded first so
the partial source receives later sequence numbers. Unlike `poison`, this preserves unrelated reuse.

### 3.6 Bounding retained residue

Alignment scans retained residue. In a measured 30-statement notebook, editing and realizing a
middle line 200 times retained 3,000 executions and increased alignment from **52 us to 2 ms**, while
the plan stayed at 15 run steps.

`realize` and `record_partial` discard residue under two monotonic rules:

1. An entry older than every realized execution can never disturb a current witness, and the minimum
   realized sequence never decreases.
2. A later entry with an identical disturbance bound subsumes an earlier one. It targets at least as
   many witnesses and has at least as many time-valid alias edges.

The second rule does most of the work in edit loops. Canonically different statements such as
`x = 1` and `x = 2` may share the same bound and safely subsume one another. Comparing canonical code
would both leak entries and mishandle calls separated by a redefinition. With bound-based compaction,
the measured residue stopped at 15 entries and alignment time stopped growing with cycle count.

Only execution references are discarded. Def-use edges and callable summaries remain because they
can widen future bounds; deleting them could permit incorrect reuse. Source storage is released when
its last retained execution reference is dropped.

## 4. Correctness argument

Let the realized sequence be `H = h1..hn`, retained residue be `R`, input be `P = p1..pm`, and
`r = prefix_len(H, P)`. Let `D+` be the time-aware alias closure of residue disturbance. Define:

```text
d(j) = Reuse iff j < r and produces+(p_j) intersect D+ is empty
d(j) = Run otherwise
```

Under the four assumptions in section 9, if `P` is self-contained—every read is a builtin or is
bound earlier in `P`—executing `Run` steps in increasing source order realizes `P`.

1. **Prefix identity.** For `i < r`, `canon(h_i) = canon(p_i)`. Linearity and A-Det make `h1..hr`
   valid executions of the same source prefix from the same initial state.
2. **Disturbance upper bound.** Syntactic binds, deletes, mutation candidates, transitive session
   summaries, time-aware aliases, and opaque fallback cover every allowed later effect under
   A-NoAlias, A-NoForeignGlobalWrite, and A-KnownEffects.
3. **Reuse case.** An empty intersection means the actual prefix execution exists and no permitted
   later effect damaged what it produced, so skipping it preserves the realized state.
4. **Run case.** Run steps execute in source order. If a read name was disturbed and an earlier source
   statement produces it, that producer also intersects disturbance and runs first. If it was not
   disturbed, its existing value remains valid. A name never bound by `P` violates self-containment;
   PySASH reports `UnresolvedReference` and the guarantee becomes session-relative.

## 5. Measured counterexamples

These cases were checked against Python behavior and are covered by regression tests.

| Case | Observed result | Alignment response |
|---|---|---|
| Late binding: `H=[K=10, def f, K=20, y=f()]`, remove `K=20` | `y=20` after restoration | Residue rebinds `K`; rerun `K=10`, reuse `def f`, then run the call. |
| Argument mutation: `a=[]; def add; add(a); n=len(a)`, remove `add(a)` | `n=0` | `mutates_params[0]` disturbs `a`; rerun its producer. |
| Decorator registry: `routes=[]; def register; @register def hello` | Empty registry after removal | The transitive summary mutates `routes`; rerun its producer. |
| Transitive global write: `def g; def h; c=0; h(); z=c`, remove `h()` | `c=0`, `z=0` | `summary*(h).global_writes={c}`; rerun `c=0`. |
| Multiplicity: `add(a)` appears twice | `n=2` | Positional witnesses preserve both executions. |
| Container alias: `a=[]; keep=a; keep.append(1)`, keep only `a=[]` | `a=[]` | Alias closure carries mutation from `keep` to `a`; rerun `a=[]`. |
| Inheritance: `class B; class C(B); C.items.append(1)` | `B.items` restored | The `(C,B)` edge carries the mutation; rerun the base class statement. |
| Module attribute: `import config; from config import LIMIT; config.LIMIT=99` | Imported `LIMIT` unchanged | Mutation of `config` does not rebind `LIMIT`; reuse the import-from statement. |
| Builtins monkeypatch | Normal builtin restored after removing patch | Reflective residue becomes opaque and forces all `Run`. |
| Identical source | Every statement reused | Full prefix and empty residue. |
| Self accumulation: append one `acc = acc + 1` | One additional increment | Reuse the existing prefix and run only the appended statement. |
| Nondeterminism: `x = random()` | Existing sampled value | The prior execution is the source execution; caller policy may downgrade it. |

## 6. Static-analysis bounds

PySASH does not model the heap or build SSA. It computes an upper bound on names each top-level
statement may affect.

### 6.1 Compound statements are atomic

`if`, `for`, `while`, `try`, `with`, `match`, `def`, and `class` each form one node. Binds are the
union of branch may-defs, and mentions include every nested name. Canonical identity covers the
entire nested body, so any internal edit changes the node. This avoids phi nodes and class-scope
corner cases, at the cost of forbidding partial reuse inside a loop or function definition.

### 6.2 Mutation and aliases

```text
mutates(s) = roots of attribute/subscript stores and augmented assignments
             U receivers and arguments of calls outside the pure whitelist
             U transitive mutates_frees from session-defined callees
             U arguments passed at mutates_params positions
```

Merely passing `x` to unknown `f(x)` makes `x` a mutation candidate. This covers the measured
`add(a)` counterexample.

Alias edges are created only for:

| Syntax | Edge |
|---|---|
| `b = a` with a bare-name right side | `(b, a)` |
| `class C(Base)` | `(C, Base)` |

Indirect aliases such as `c = [a]`, `d = f(a)`, and `return self._data` are excluded by A-NoAlias.
Unioning every read with every definition quickly collapses the namespace into one alias class and
eliminates useful reuse, so the boundary is deliberately narrow.

### 6.3 Callable summaries

Session-defined functions and classes record `global_writes`, `mutates_frees`, mutated positional
parameters, named callees, and opacity. Calls resolve their transitive closure with a visited set, so
recursion and mutual recursion terminate. Imported functions have no body summary; the gap is covered
by A-NoForeignGlobalWrite and the conservative mutation bound on mentioned objects.

## 7. Canonicalization boundary

Over-normalization can authorize incorrect reuse; under-normalization only adds execution. PySASH
therefore uses exactly Ruff's `ComparableStmt` boundary plus docstring position and a schema tag.

| Normalized | Kept distinct |
|---|---|
| Whitespace, comments, line endings, redundant parentheses | `1000` vs `1000.0`; `True` vs `1` |
| Equivalent integer, string, bytes, and implicit-concatenation spellings | f-strings with different runtime formatting |
| Tuple parentheses and trailing commas | `f"{a=}"` vs `f"{ a = }"` |
| One-line vs multiline suites | Alpha-equivalent parameter names |
| NFKC identifiers as normalized by CPython and Ruff | Constant folding such as `2*500` vs `1000` |

A bare string is a docstring only at position zero, so `is_docstring` participates in the encoding.
The schema includes the pinned Ruff version. Ruff's comparable representation omits expression
context; identity therefore hashes whole statements, while fact extraction uses the original AST
where `Load`, `Store`, and `Del` remain available.

`from __future__` needs no extra canonical state. If its statement remains in the prefix, both sides
executed the same directive; otherwise alignment runs from the divergence point.

## 8. Dynamic constructs

| Construct | Treatment |
|---|---|
| Reflective names (`exec`, `eval`, `globals`, `locals`, `vars`, dynamic `getattr`, `setattr`, `delattr`, `__import__`, `importlib`, `builtins`) and reflective attributes | Mark opaque; opaque residue forces all `Run`. |
| `from m import *` | Mark opaque because its binding set is unknown. |
| `del x` | Record in `deletes`. |
| Compound statements | Treat as one atomic node. |
| `def` and `class` | Atomic node plus callable summary. |
| Conditional import | Atomic compound statement with the union of possible bindings. |
| Decorator | Record mentioned names and calls at definition time. |
| Class inheritance | Add an alias edge for shared mutable attributes. |
| Function `global` write | Record in the callable summary and resolve transitively. |
| Unknown `f(x)` | Treat `x` as possibly mutated; assume no foreign global rebind. |
| Unknown `x.m(y)` | Treat receiver and arguments as possibly mutated. |
| IPython magic | Return `ParseError`; only Python syntax is accepted. |
| PEP 263 non-UTF-8 cookie | Return `UnsupportedEncoding` so ranges keep one byte basis. |

## 9. Exactly four assumptions

Static analysis cannot soundly model arbitrary Python without assumptions. PySASH names exactly the
four contracts on which reuse depends.

| Name | Assumption | Failure mode | Mitigation |
|---|---|---|---|
| **A-Det** | Canonically equal statements executed in the same order from the same state produce the same state. | Prefix identity becomes invalid. | Nondeterminism is already realized; external changes are exposed through `Effect` for caller policy. |
| **A-NoAlias** | Objects change only through syntactically mentioned names, bare-name assignment aliases, or class inheritance. | An indirect mutation may permit incorrect reuse. | Track the two bounded alias forms and document the rest as unsupported. |
| **A-NoForeignGlobalWrite** | A function defined outside the session does not rebind this module's globals. | A hidden global rebind may permit incorrect reuse. | Python `global` normally targets the defining module; visible reflection becomes opaque. |
| **A-KnownEffects** | An unknown call stays within result bindings, mentioned-object mutation, and transitive summaries of session-defined callees. | Hidden higher-order effects may permit incorrect reuse. | Treat every mentioned receiver and argument as a mutation candidate. |

## 10. Explicitly out of scope for v0.1

1. Indirect aliases: `c = [a]`, `d = f(a)`, `return self._data`, `dict.setdefault`, unpacked aliases.
2. Higher-order calls and dynamic dispatch: `h = f; h()`, `fs[0]()`, dynamic `getattr` calls.
3. Interpreter flags such as `-O`, `-OO`, and `PYTHONHASHSEED`.
4. C-extension internal state and external files changed outside the process.
5. Partial reuse inside a compound statement.
6. Persistent histories; the schema tag only prepares canonical encodings for invalidation.
7. Fork and rewind; the session model is linear.

PySASH also does not represent progress within one compound statement. If a loop fails halfway,
`record_partial` adds the complete containing source to residue. The loss is precision, not safety
under the four assumptions.

## 11. Why the Ruff crates are pinned exactly

`ruff_python_parser`, `ruff_python_ast`, and `ruff_text_size` are exact-version dependencies because
the 0.0.x releases are semver-incompatible and duplicate AST versions have incompatible Rust types.
No Ruff type appears in PySASH's public API, so parser changes can be absorbed in a PySASH patch
release. A parser upgrade also changes the canonical schema tag, preventing old identities from
surviving silently.

The cost is an MSRV near current stable Rust. `rust-toolchain.toml` is authoritative. If the Ruff
crates later offer stable semver compatibility, the dependency range can be relaxed.
