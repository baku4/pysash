# tests/fixtures — alignment fixtures drawn from real code

Every `.py` here is jupytext `py:percent` and is **never executed**. PySASH is static
analysis, so a fixture only needs to be parseable Python. Paths, filenames, and environment
names are left as they were in the original code on purpose: the point is to see how the
verdicts come out on real research code, not on tidied examples.

## `corpus/` — extracted from real research code

Excerpts from three bioinformatics projects (read-mapping benchmarks, a bacterial pangenome
pipeline, and a k-mer analysis toolkit). Selection criteria:

1. Pure Python — any IPython magic (`%`, `!`, `?`) is a `ParseError` (see README, out of scope).
2. One whole workflow per file — import → paths → load → transform → save/plot.
3. Non-overlapping character — notebooks, percent scripts, and plain modules.

| File | Character |
|---|---|
| `notebook_define_file_paths.py` | path/config definitions, `import *`, helper defs |
| `notebook_compare_tools.py` | accumulating list → `pd.concat` → table cleanup |
| `notebook_simulate_reads.py` | assembling external simulator commands, batch execution, a large `for` loop |
| `notebook_merge_lineage.py` | pandas merge/rename/apply |
| `notebook_contig_length_qc.py` | decorators, multiprocessing, plotly |
| `notebook_upload_assembly.py` | file writes, nested `with` |
| `notebook_gwas_manhattan.py` | a large plotting function called repeatedly |
| `notebook_variants_table.py` | assembling a variant table |
| `script_run_skani.py` | already percent-formatted at the source |
| `script_download_assemblies.py` | already percent-formatted at the source |
| `script_pairwise_ani.py` | already percent-formatted at the source |
| `module_metrics.py` | docstrings + pure functions |
| `module_plotting.py` | matplotlib functions |
| `module_tools.py` | relative imports, subprocess |

These files are not edited by hand. Cell boundaries (`# %%`) are plain comments to PySASH and
do not affect any verdict; they are kept to record how the original code was executed.

Two test files use `corpus/`, and neither carries hand-written expected values:

- `tests/corpus.rs` — properties that must hold for any input.
- `tests/reuse.rs` — a baseline of cell-level reuse rates. These are measurements, not values
  derived from the rules, so they fail when precision improves; updating the table is the
  evidence of the improvement.

## `sessions/` — editing scenarios

Each directory replays a shell session where the user edits a source up and down. `01_base.py`
is the source that ran first; the rest are its edited versions. **One top-level statement per
cell**, so cell numbers and statement indices match 1:1 and the expected values can be followed
by hand.

| Scenario | Based on | What it checks |
|---|---|---|
| `contig_qc/` | `notebook_contig_length_qc.py` | append / edit the last line / edit a constant above / insert / delete / reorder / reformat |
| `notebook_prologue/` | a notebook prologue from the corpus | `from … import *` inside vs outside the prefix |
| `merge_results/` | `notebook_compare_tools.py` | whether in-place list accumulation reruns the producer |
| `gwas_labeling/` | `notebook_gwas_manhattan.py` | editing a helper below — `Run` is not a contiguous range |

Expected values live in `tests/editing.rs`, not in the fixtures. They are derived by hand from
the decision rules, not observed from the implementation — copying what the implementation does
would test nothing.
